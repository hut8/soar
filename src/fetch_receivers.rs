use chrono::{SecondsFormat, Utc};
use clap::Parser;
use once_cell::sync::Lazy;
use rand::RngExt;
use regex::{Captures, Regex};
use reqwest::header::{CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap, fs::File, io::Write};
use tracing::{debug, info};

static WIKI_URL: &str = "http://wiki.glidernet.org/ajax-module-connector.php";

/// Matches '++ <text> [[<tag>' headings (country sections)
static HEADING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\+\+ (?P<text>.*) ?\[\[(?P<tag>.*)$").unwrap());

/// Table row for receiver lines
static RECEIVER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?xm)
        ^\|\|\ \|\|\ \[\[\#\ ?(?P<aprsname>.*?)\]\].*?\|\|
        (?P<desc>.*?)\|\|
        (?P<photos>.*?)\|\|
        .*?\|\|
        (?P<contact>.*?)\|\|(?:<br\s*/?>)?\s*$
    ",
    )
    .unwrap()
});

/// [*href label] style links used in description
static WIKIDOT_LINK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"\[\*?([^\[\]\ ]*)\ ([^\[\]]*)\]"#).unwrap());

/// [*photo_url name]
static PHOTOS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"\[\*?(?P<photo_url>[^\s\[\]]*)\s+(?P<name>[^\]]*)\]"#).unwrap());

static IMAGE_URL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i).*\.(svg|jpeg|pdf|apng|mng|jpg|png|gif)$").unwrap());

static MAIL_ADDRESS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^[a-z0-9+\-_%.]+@[a-z0-9+\-_%.]+\.[a-z]{2,}$").unwrap());

static CONTACT_MAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\[\[\[mailto:(?P<email>[^?\ ]*)(?P<subject>.*)\|\ *(?P<name>.*)\]\]\]"#).unwrap()
});

static CONTACT_URL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"\[\[\[(?P<url>http.*)\|(?P<name>.*)\]\]\]"#).unwrap());

static CONTACT_INTERN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?xm)
        \[\/contact\ (?P<name0>\S*)
        (?:
          \]\s\/\s\[\/contact\ (?P<name1>.*)\] |
          \]\s\/\s(?P<name2>.*) |
          \]
        )
    "#,
    )
    .unwrap()
});

static PHOTOS_BASE_URL: &str = "http://openglidernetwork.wdfiles.com";

static RECEIVER_LIST_PAGE_IDS: Lazy<HashMap<&'static str, i64>> = Lazy::new(|| {
    HashMap::from([
        ("others", 22120125),
        ("france", 45174721),
        ("germany", 45177548),
        ("uk", 45177553),
        ("us", 45426379),
    ])
});

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Receiver {
    callsign: String,
    description: String,
    photos: Vec<String>,
    links: Vec<Link>,
    contact: String,
    email: String,
    country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Link {
    href: String,
    rel: Option<String>,
}

#[derive(Debug, Serialize)]
struct Output {
    version: &'static str,
    receivers: Vec<Receiver>,
    timestamp: String,
}

#[derive(Debug, Deserialize)]
struct WikidotResp {
    body: String,
}

#[derive(Parser, Debug)]
#[command(
    name = "ogn-receiver-scraper",
    about = "Fetch list-of-receivers from wiki.glidernet.org and output JSON."
)]
struct Args {
    /// Output file (default: receivers.json)
    #[arg(long = "out", default_value = "receivers.json")]
    out_file: String,
    /// Obfuscate emails (truncate/blank out email addresses)
    #[arg(long = "obfuscate", default_value_t = false)]
    obfuscate: bool,
}

fn gen_token() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

async fn fetch_page(client: &reqwest::Client, url: &str, page_id: i64) -> reqwest::Result<String> {
    let token = gen_token();

    // Build headers including cookie
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("wikidot_token7={}", token)).unwrap(),
    );

    let params = [
        ("page_id", page_id.to_string()),
        ("moduleName", "viewsource/ViewSourceModule".to_string()),
        ("callbackIndex", "1".to_string()),
        ("wikidot_token7", token.clone()),
    ];

    let resp = client
        .post(url)
        .headers(headers)
        .form(&params)
        .send()
        .await?
        .error_for_status()?
        .json::<WikidotResp>()
        .await?;

    debug!("Fetched page_id={}:\n{}", page_id, resp.body);

    Ok(resp.body)
}

fn parse_description_links(mut text: String) -> (String, Vec<Link>) {
    let mut links: Vec<Link> = Vec::new();

    // Replace each [*href label] with just "label" and collect links
    let replaced = WIKIDOT_LINK_RE.replace_all(&text, |caps: &Captures| {
        let href = caps.get(1).unwrap().as_str().to_string();
        let rel = caps.get(2).unwrap().as_str().to_string();
        links.push(Link {
            href,
            rel: Some(rel.clone()),
        });
        Cow::Owned(rel) // replace token with rel (label)
    });

    text = replaced.into_owned();
    (text, links)
}

fn parse_contact(raw: &str) -> (String, String, Vec<Link>) {
    let mut links: Vec<Link> = Vec::new();

    if let Some(c) = CONTACT_MAIL_RE.captures(raw) {
        let email = c.name("email").map(|m| m.as_str()).unwrap_or("").trim();
        let name = c.name("name").map(|m| m.as_str()).unwrap_or("").trim();
        if MAIL_ADDRESS_RE.is_match(email) {
            return (name.to_string(), email.to_string(), links);
        } else {
            return (name.to_string(), String::new(), links);
        }
    } else if let Some(c) = CONTACT_URL_RE.captures(raw) {
        let url = c.name("url").map(|m| m.as_str()).unwrap_or("").to_string();
        let name = c.name("name").map(|m| m.as_str()).unwrap_or("").to_string();
        links.push(Link {
            href: url,
            rel: Some("contact".to_string()),
        });
        return (name, String::new(), links);
    } else if let Some(c) = CONTACT_INTERN_RE.captures(raw) {
        // Join name0/name1/name2 if present
        let mut parts = vec![];
        for key in ["name0", "name1", "name2"] {
            if let Some(m) = c.name(key) {
                let v = m.as_str().trim();
                if !v.is_empty() {
                    parts.push(v);
                }
            }
        }
        return (parts.join(" / "), String::new(), links);
    } else {
        let name = raw.replace(['[', ']', '|'], "").trim().to_string();
        if !name.is_empty() {
            return (name, String::new(), links);
        }
    }
    (String::new(), String::new(), links)
}

fn parse_photo_links(raw: &str) -> (Vec<String>, Vec<Link>) {
    let mut photos = Vec::new();
    let mut links = Vec::new();

    for caps in PHOTOS_RE.captures_iter(raw) {
        let url = caps.name("photo_url").map(|m| m.as_str()).unwrap_or("");
        let name = caps.name("name").map(|m| m.as_str()).unwrap_or("");

        if url.starts_with("/local--files") {
            photos.push(format!("{}{}", PHOTOS_BASE_URL, url));
        } else if IMAGE_URL_RE.is_match(url) {
            photos.push(url.to_string());
        } else {
            links.push(Link {
                href: url.to_string(),
                rel: Some(name.to_string()),
            });
        }
    }
    (photos, links)
}

fn html_unescape(s: &str) -> String {
    s.replace("&nbsp;", "")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}

fn parse_receiver_list(page: &str) -> Vec<Receiver> {
    // Split into lines grouped by current "country heading"
    let mut current_country = String::from("None");
    let mut by_country: HashMap<String, String> = HashMap::new();
    by_country.insert(current_country.clone(), String::new());

    for line in page.lines() {
        if let Some(cap) = HEADING_RE.captures(line) {
            let text = cap.name("text").unwrap().as_str().trim().to_string();
            current_country = text.to_lowercase();
            by_country.entry(current_country.clone()).or_default();
        } else {
            by_country
                .entry(current_country.clone())
                .and_modify(|buf| {
                    buf.push_str(line);
                    buf.push('\n');
                })
                .or_insert_with(|| {
                    let mut s = String::new();
                    s.push_str(line);
                    s.push('\n');
                    s
                });
        }
    }

    let mut receivers = Vec::new();

    for (country_name, blob) in by_country {
        for line in blob.lines() {
            let line = html_unescape(line);
            if let Some(m) = RECEIVER_RE.captures(&line) {
                let callsign = m.name("aprsname").unwrap().as_str().to_string();

                let desc_raw = m.name("desc").unwrap().as_str().trim().to_string();
                let (description, mut desc_links) = parse_description_links(desc_raw);

                let photos_raw = m.name("photos").unwrap().as_str();
                let (photos, mut photo_links) = parse_photo_links(photos_raw);

                let contact_raw = m.name("contact").unwrap().as_str();
                let (contact, email, mut contact_links) = parse_contact(contact_raw);

                let mut links = Vec::new();
                links.append(&mut desc_links);
                links.append(&mut photo_links);
                links.append(&mut contact_links);

                receivers.push(Receiver {
                    callsign,
                    description,
                    photos,
                    links,
                    contact,
                    email,
                    country: Some(country_name.clone()),
                });
            }
        }
    }

    receivers
}

pub async fn fetch_receivers(out_file: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("ogn-receiver-scraper/0.2")
        .build()?;

    info!("Fetch and parse lists of receivers");
    let mut all: Vec<Receiver> = Vec::new();

    for (country_key, page_id) in RECEIVER_LIST_PAGE_IDS.iter() {
        debug!(country_key, page_id);
        let page = fetch_page(&client, WIKI_URL, *page_id).await?;
        let receivers = parse_receiver_list(&page);

        // Country names are now stored verbatim from the wiki headings
        // No normalization to ISO codes is performed
        all.extend(receivers);
    }

    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let out = Output {
        version: "0.2.1",
        receivers: all,
        timestamp: ts,
    };

    info!("Saving receivers to {}", out_file);
    let mut f = File::create(out_file)?;
    let json = serde_json::to_string(&out)?;
    f.write_all(json.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_receiver_list_with_fixture() {
        // Read the test fixture
        let fixture_content = std::fs::read_to_string("tests/fixtures/ogn/receivers-page.html")
            .expect("Failed to read test fixture");

        let receivers = parse_receiver_list(&fixture_content);

        // Debug output to see what we get
        eprintln!("Found {} receivers", receivers.len());
        for (i, receiver) in receivers.iter().take(5).enumerate() {
            eprintln!(
                "Receiver {}: callsign='{}', description='{}', country='{}'",
                i,
                receiver.callsign,
                receiver.description,
                receiver.country.as_deref().unwrap_or("None")
            );
        }

        // We should find multiple receivers
        assert!(!receivers.is_empty(), "Should find at least one receiver");
    }

    #[test]
    fn test_receiver_regex_with_sample_line() {
        let sample_line = r#"|| || [[# Agathazel]][*http://ognrange.onglide.com/#Agathazel Agathazel] || Agathazell glider airfield. [*http://www.agathazell.info Luftsportverein Agathazell]|| [*http://www.agathazell.info/images/joomlaplates/Banner.jpg Airfield] [*http://agathazell.info/images/joomlaplates/Open-Glider-Network-Antenna-Agathazell.JPG Antenna] [*http://agathazell.info/images/joomlaplates/Open-Glider-Network-Receiver-Agathazell.jpg Receiver] [*http://agathazell.info/images/joomlaplates/Open-Glider-Network-Antenna2-Agathazell.JPG.jpg Antenna2]|| [[image http://glidertracking.fai.org/availability.php?station=Agathazel]] || [[[mailto:1.vorstand.lsg@agathazell.info?subject=OGN receiver: Agathazel | LSG Oberallgäu e.V.]]] ||<br />"#;

        let captures = RECEIVER_RE.captures(sample_line);
        match captures {
            Some(caps) => {
                let callsign = caps.name("aprsname").unwrap().as_str();
                let desc = caps.name("desc").unwrap().as_str().trim();
                let photos = caps.name("photos").unwrap().as_str();
                let contact = caps.name("contact").unwrap().as_str();

                eprintln!("Callsign: '{}'", callsign);
                eprintln!("Description: '{}'", desc);
                eprintln!("Photos: '{}'", photos);
                eprintln!("Contact: '{}'", contact);

                assert_eq!(callsign, "Agathazel");
                assert!(desc.contains("Agathazell glider airfield"));
            }
            None => {
                eprintln!("Regex did not match sample line");
                panic!("RECEIVER_RE should match the sample line");
            }
        }
    }

    #[test]
    fn test_heading_regex() {
        let heading_samples = vec![
            "++ Germany [[germany",
            "++ France [[france",
            "++ United Kingdom [[uk",
        ];

        for sample in heading_samples {
            let captures = HEADING_RE.captures(sample);
            match captures {
                Some(caps) => {
                    let text = caps.name("text").unwrap().as_str();
                    let tag = caps.name("tag").unwrap().as_str();
                    eprintln!("Heading - Text: '{}', Tag: '{}'", text, tag);
                }
                None => {
                    eprintln!("Heading regex did not match: '{}'", sample);
                }
            }
        }
    }

    #[test]
    fn test_contact_parsing() {
        let test_cases = vec![
            "[[[mailto:1.vorstand.lsg@agathazell.info?subject=OGN receiver: Agathazel | LSG Oberallgäu e.V.]]]",
            "[[[mailto:domspreitz@googlemail.com?subject=OGN receiver: Altdorf | Dominic]]]",
            "[[[http://example.com | Example Contact]]]",
        ];

        for test_case in test_cases {
            let (contact, email, links) = parse_contact(test_case);
            eprintln!(
                "Contact parsing - Input: '{}' -> Contact: '{}', Email: '{}', Links: {:?}",
                test_case, contact, email, links
            );
        }
    }

    #[test]
    fn test_photo_parsing() {
        let test_cases = vec![
            "[*http://www.agathazell.info/images/joomlaplates/Banner.jpg Airfield] [*http://agathazell.info/images/joomlaplates/Open-Glider-Network-Antenna-Agathazell.JPG Antenna]",
            "[*https://sfvbw.de/wp-content/uploads/OGN-Antenne_1.jpg Antenna1] [*https://sfvbw.de/wp-content/uploads/OGN-Antenne_2.jpg Antenna2]",
        ];

        for test_case in test_cases {
            let (photos, links) = parse_photo_links(test_case);
            eprintln!(
                "Photo parsing - Input: '{}' -> Photos: {:?}, Links: {:?}",
                test_case, photos, links
            );
        }
    }

    #[test]
    fn test_receiver_structure_validation() {
        // Read the test fixture
        let fixture_content = std::fs::read_to_string("tests/fixtures/ogn/receivers-page.html")
            .expect("Failed to read test fixture");

        let receivers = parse_receiver_list(&fixture_content);

        // Validate first receiver in detail
        assert!(!receivers.is_empty(), "Should have at least one receiver");

        let first_receiver = &receivers[0];
        assert_eq!(first_receiver.callsign, "Agathazel");
        assert!(!first_receiver.description.is_empty());
        assert!(!first_receiver.contact.is_empty());
        assert!(!first_receiver.email.is_empty());
        assert!(!first_receiver.photos.is_empty(), "Should have photos");

        // Validate some photos are proper URLs
        for photo in &first_receiver.photos {
            assert!(
                photo.starts_with("http"),
                "Photo should be a URL: {}",
                photo
            );
        }

        // Check that emails are valid format
        let email_regex =
            regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        assert!(
            email_regex.is_match(&first_receiver.email),
            "Email should be valid format: {}",
            first_receiver.email
        );

        eprintln!("First receiver validation passed:");
        eprintln!("  Callsign: {}", first_receiver.callsign);
        eprintln!("  Description: {}", first_receiver.description);
        eprintln!("  Contact: {}", first_receiver.contact);
        eprintln!("  Email: {}", first_receiver.email);
        eprintln!("  Photos: {:?}", first_receiver.photos);
        eprintln!("  Links: {:?}", first_receiver.links);
    }
}

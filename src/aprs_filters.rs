use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct FilterExpr {
    pub terms: Vec<FilterItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterItem {
    pub negated: bool,
    pub kind: FilterKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterKind {
    /// r/<lat>/<lon>/<range_km>
    Range { lat: f64, lon: f64, km: f64 },

    /// a/<lat1>/<lon1>/<lat2>/<lon2>
    Area {
        lat1: f64,
        lon1: f64,
        lat2: f64,
        lon2: f64,
    },

    /// b/<call1>/<call2>/... (accepts wildcards like FLR* or OGN*)
    Buddies(Vec<String>),

    /// g/<group> (e.g., g/ALL)
    Group(String),

    /// t/<letters> (packet types); accepts freeform letters/symbols you provided (e.g., n, p, s, spuoimnwt)
    TypeSet(String),

    /// p/<prefix1>/<prefix2>/... (station prefixes)
    Prefixes(Vec<String>),

    /// s/<symbol-table>/<symbol-code> (or other symbol-ish combos; we don't enforce strict APRS here)
    Symbol(String, String),

    /// u/<prefix1>/<prefix2>/... (often IGate/TOCALL prefixes; we keep as list)
    UPrefixes(Vec<String>),

    /// e/<dest1>/<dest2>/... (destination/address patterns)
    Destinations(Vec<String>),

    /// m/<km> (range around “my position” on server side; stored, not interpreted)
    MyRangeKm(f64),

    /// Raw / unknown token preserved as-is (e.g., "lzma", malformed r///, etc.)
    Unknown(String),

    /// A token that failed structured parsing with reason; original preserved
    Invalid { original: String, reason: String },
}

#[derive(Debug)]
pub struct ParseFilterError {
    pub message: String,
}

impl Display for ParseFilterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ParseFilterError {}

impl FromStr for FilterExpr {
    type Err = ParseFilterError;

    /// Parse a full filter string like:
    ///   "g/ALL r/54.1989/80.2397/150.0 r/52.9537/41.4655/100.0 -t/n b/OGN*/FLR* lzma"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut terms = Vec::new();

        for raw in tokenize_by_spaces(s) {
            if raw.trim().is_empty() {
                continue;
            }
            let (negated, token) = if let Some(stripped) = raw.strip_prefix('-') {
                (true, stripped)
            } else {
                (false, raw.as_str())
            };

            let item = parse_single_token(token).unwrap_or_else(|e| FilterKind::Invalid {
                original: token.to_string(),
                reason: e,
            });

            terms.push(FilterItem {
                negated,
                kind: item,
            });
        }

        Ok(FilterExpr { terms })
    }
}

impl Display for FilterExpr {
    /// Serialize back to an APRS filter string (keeps order and negation).
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::with_capacity(self.terms.len());
        for t in &self.terms {
            let s = match &t.kind {
                FilterKind::Range { lat, lon, km } => format!(
                    "r/{}/{}/{}",
                    fmt_float(*lat),
                    fmt_float(*lon),
                    fmt_float(*km)
                ),
                FilterKind::Area {
                    lat1,
                    lon1,
                    lat2,
                    lon2,
                } => format!(
                    "a/{}/{}/{}/{}",
                    fmt_float(*lat1),
                    fmt_float(*lon1),
                    fmt_float(*lat2),
                    fmt_float(*lon2)
                ),
                FilterKind::Buddies(list) => format!("b/{}", list.join("/")),
                FilterKind::Group(g) => format!("g/{}", g),
                FilterKind::TypeSet(syms) => format!("t/{}", syms),
                FilterKind::Prefixes(pref) => format!("p/{}", pref.join("/")),
                FilterKind::Symbol(tbl, code) => format!("s/{}/{}", tbl, code),
                FilterKind::UPrefixes(pref) => format!("u/{}", pref.join("/")),
                FilterKind::Destinations(dest) => format!("e/{}", dest.join("/")),
                FilterKind::MyRangeKm(km) => format!("m/{}", fmt_float(*km)),
                FilterKind::Unknown(s) => s.clone(),
                FilterKind::Invalid { original, .. } => original.clone(),
            };
            parts.push(if t.negated { format!("-{}", s) } else { s });
        }
        write!(f, "{}", parts.join(" "))
    }
}

// ---------------------------- helpers ----------------------------

fn tokenize_by_spaces(s: &str) -> Vec<String> {
    // Keep it simple: split on whitespace; APRS filters are space-separated
    s.split_whitespace().map(|x| x.to_string()).collect()
}

fn parse_single_token(tok: &str) -> Result<FilterKind, String> {
    // Identify <tag>/<...> pattern; if not, treat as Unknown unless it's obviously unparseable r///
    let mut parts = tok.splitn(2, '/');
    let head = parts.next().unwrap_or("");
    let tail = parts.next(); // None if no '/'

    match (head, tail) {
        ("r", Some(rest)) => parse_r(rest),
        ("a", Some(rest)) => parse_a(rest),
        ("b", Some(rest)) => Ok(FilterKind::Buddies(split_nonempty(rest))),
        ("g", Some(rest)) => Ok(FilterKind::Group(rest.to_string())),
        ("t", Some(rest)) => Ok(FilterKind::TypeSet(rest.to_string())),
        ("p", Some(rest)) => Ok(FilterKind::Prefixes(split_nonempty(rest))),
        ("s", Some(rest)) => parse_s(rest),
        ("u", Some(rest)) => Ok(FilterKind::UPrefixes(split_nonempty(rest))),
        ("e", Some(rest)) => Ok(FilterKind::Destinations(split_nonempty(rest))),
        ("m", Some(rest)) => {
            let km = parse_f64(rest).map_err(|e| format!("bad m/<km>: {}", e))?;
            Ok(FilterKind::MyRangeKm(km))
        }
        // Bare "r///" edge case still hits ("r", Some(rest))
        (tag, None) => Ok(FilterKind::Unknown(tag.to_string())),
        _ => Ok(FilterKind::Unknown(tok.to_string())),
    }
}

fn parse_r(rest: &str) -> Result<FilterKind, String> {
    // Expect: lat/lon/range_km
    let v = split_allow_empty(rest);
    if v.len() != 3 {
        return Err(format!("bad r/<lat>/<lon>/<km>: got {} parts", v.len()));
    }
    let lat = parse_f64(&v[0]).map_err(|e| format!("r/lat: {}", e))?;
    let lon = parse_f64(&v[1]).map_err(|e| format!("r/lon: {}", e))?;
    let km = parse_f64(&v[2]).map_err(|e| format!("r/km: {}", e))?;
    validate_lat_lon(lat, lon)?;
    if !km.is_finite() || km < 0.0 {
        return Err("r/km must be finite, >= 0".to_string());
    }
    Ok(FilterKind::Range { lat, lon, km })
}

fn parse_a(rest: &str) -> Result<FilterKind, String> {
    // Expect: lat1/lon1/lat2/lon2 (rectangle corners)
    let v = split_allow_empty(rest);
    if v.len() != 4 {
        return Err(format!(
            "bad a/<lat1>/<lon1>/<lat2>/<lon2>: got {} parts",
            v.len()
        ));
    }
    let lat1 = parse_f64(&v[0]).map_err(|e| format!("a/lat1: {}", e))?;
    let lon1 = parse_f64(&v[1]).map_err(|e| format!("a/lon1: {}", e))?;
    let lat2 = parse_f64(&v[2]).map_err(|e| format!("a/lat2: {}", e))?;
    let lon2 = parse_f64(&v[3]).map_err(|e| format!("a/lon2: {}", e))?;
    validate_lat_lon(lat1, lon1)?;
    validate_lat_lon(lat2, lon2)?;
    Ok(FilterKind::Area {
        lat1,
        lon1,
        lat2,
        lon2,
    })
}

fn parse_s(rest: &str) -> Result<FilterKind, String> {
    // We’ll accept s/<table>/<code>, where each may be 1+ chars (some examples have single-letter like s/g or s/z)
    let v = split_allow_empty(rest);
    if v.is_empty() {
        return Err("s/ requires at least one part".to_string());
    }
    if v.len() == 1 {
        // Keep single part as table with empty code
        return Ok(FilterKind::Symbol(v[0].clone(), String::new()));
    }
    Ok(FilterKind::Symbol(v[0].clone(), v[1].clone()))
}

fn split_allow_empty(s: &str) -> Vec<String> {
    s.split('/').map(|x| x.to_string()).collect()
}

fn split_nonempty(s: &str) -> Vec<String> {
    s.split('/')
        .filter(|x| !x.is_empty())
        .map(|x| x.to_string())
        .collect()
}

fn parse_f64(s: &str) -> Result<f64, String> {
    if s.is_empty() {
        return Err("missing number".to_string());
    }
    s.parse::<f64>().map_err(|_| format!("not a number: {}", s))
}

fn validate_lat_lon(lat: f64, lon: f64) -> Result<(), String> {
    if !lat.is_finite() || !lon.is_finite() {
        return Err("lat/lon must be finite".to_string());
    }
    if !(-90.0..=90.0).contains(&lat) {
        return Err(format!("lat out of range [-90,90]: {}", lat));
    }
    if !(-180.0..=180.0).contains(&lon) {
        return Err(format!("lon out of range [-180,180]: {}", lon));
    }
    Ok(())
}

fn fmt_float(x: f64) -> String {
    // Trim trailing zeros while keeping a decimal if needed
    let s = format!("{}", x);
    if s.contains('.') {
        let t = s.trim_end_matches('0').trim_end_matches('.');
        t.to_string()
    } else {
        s
    }
}

// ---------------------------- demo & tests ----------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_samples() {
        let s = "g/ALL r/54.1989/80.2397/150.0 r/52.9537/41.4655/100.0 -t/n \
                 b/OGN22244E/OGN3B232F/OGN88CD3E a/49.1678/13.9455/48.9972/14.2061 \
                 r/54.9331316095147/-1.6185537037205713/10 t/spuoimnwt lzma r///";
        let expr = FilterExpr::from_str(s).unwrap();
        assert!(!expr.terms.is_empty());

        // spot-check a few
        assert!(matches!(expr.terms[0].kind, FilterKind::Group(ref g) if g == "ALL"));
        assert!(matches!(expr.terms[1].kind, FilterKind::Range { .. }));
        assert!(matches!(expr.terms[5].kind, FilterKind::Area { .. }));
        // negated t/n
        assert!(
            expr.terms
                .iter()
                .any(|t| t.negated && matches!(t.kind, FilterKind::TypeSet(ref x) if x=="n"))
        );
        // unknown and invalid preserved
        assert!(
            expr.terms
                .iter()
                .any(|t| matches!(t.kind, FilterKind::Unknown(ref x) if x=="lzma"))
        );
        assert!(
            expr.terms.iter().any(
                |t| matches!(t.kind, FilterKind::Invalid{ref original, ..} if original=="r///")
            )
        );
    }

    #[test]
    fn round_trip() {
        let s = "-p/oimqstunw r/48.0/10.0/100 t/p e/LH* u/OGFLR/OGNT*";
        let expr = FilterExpr::from_str(s).unwrap();
        let back = expr.to_string();
        // Round-trip should produce a logically equivalent string
        // Note: floating point formatting may omit .0 for whole numbers
        assert_eq!(back, "-p/oimqstunw r/48/10/100 t/p e/LH* u/OGFLR/OGNT*");
    }
}

use anyhow::Result;
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;

/// Get the environment name for display purposes
/// Returns "Production", "Staging", or "Development"
fn get_environment_name() -> String {
    match std::env::var("SOAR_ENV").unwrap_or_default().as_str() {
        "production" => "Production".to_string(),
        "staging" => "Staging".to_string(),
        _ => "Development".to_string(),
    }
}

/// Get the staging prefix for email subjects
/// Returns "[STAGING] " if SOAR_ENV=staging, empty string otherwise
fn get_staging_prefix() -> &'static str {
    match std::env::var("SOAR_ENV").unwrap_or_default().as_str() {
        "staging" => "[STAGING] ",
        _ => "",
    }
}

#[derive(Debug, Clone)]
pub struct TableArchiveMetrics {
    pub table_name: String,
    pub rows_deleted: usize,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub duration_secs: f64,
    pub oldest_remaining: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct DailyCount {
    pub date: NaiveDate,
    pub count: i64,
    pub archived: bool, // true if this day was archived (pruned)
}

#[derive(Debug, Clone)]
pub struct ArchiveReport {
    pub total_duration_secs: f64,
    pub tables: Vec<TableArchiveMetrics>,
    pub daily_counts: HashMap<String, Vec<DailyCount>>, // table_name -> Vec<DailyCount>
    pub unreferenced_locations_7d: Option<i64>, // Count of unreferenced locations created in last 7 days
}

impl Default for ArchiveReport {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchiveReport {
    pub fn new() -> Self {
        Self {
            total_duration_secs: 0.0,
            tables: Vec::new(),
            daily_counts: HashMap::new(),
            unreferenced_locations_7d: None,
        }
    }

    pub fn add_table(&mut self, metrics: TableArchiveMetrics) {
        self.tables.push(metrics);
    }

    pub fn add_daily_counts(&mut self, table_name: String, counts: Vec<DailyCount>) {
        self.daily_counts.insert(table_name, counts);
    }

    fn format_duration(secs: f64) -> String {
        if secs < 60.0 {
            format!("{:.1}s", secs)
        } else if secs < 3600.0 {
            format!("{:.1}m", secs / 60.0)
        } else {
            format!("{:.1}h", secs / 3600.0)
        }
    }

    fn format_file_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn format_number(n: usize) -> String {
        let s = n.to_string();
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i).is_multiple_of(3) {
                result.push(',');
            }
            result.push(*c);
        }
        result
    }

    fn format_count(n: i64) -> String {
        let s = n.to_string();
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i).is_multiple_of(3) {
                result.push(',');
            }
            result.push(*c);
        }
        result
    }

    fn relative_time_days(date: NaiveDate) -> String {
        let today = Utc::now().date_naive();
        let days_ago = (today - date).num_days();
        if days_ago == 0 {
            "today".to_string()
        } else if days_ago == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days_ago)
        }
    }

    pub fn to_html(&self) -> String {
        let environment = get_environment_name();

        // Build summary section with optional unreferenced locations
        let mut summary_html = format!(
            r#"<strong>Environment:</strong> {}<br>
            <strong>Total Duration:</strong> {}<br>
            <strong>Tables Processed:</strong> {}<br>
            <strong>Total Rows Archived:</strong> {}<br>"#,
            environment,
            Self::format_duration(self.total_duration_secs),
            self.tables.len(),
            Self::format_number(self.tables.iter().map(|t| t.rows_deleted).sum())
        );

        if let Some(count) = self.unreferenced_locations_7d {
            summary_html.push_str(&format!(
                r#"<strong>Unreferenced Locations (last 7 days):</strong> {}<br>"#,
                Self::format_count(count)
            ));
        }

        summary_html.push_str(&format!(
            r#"<strong>Time:</strong> {}"#,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));

        let mut html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background-color: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; margin-bottom: 10px; }}
        h2 {{ color: #555; margin-top: 30px; margin-bottom: 15px; border-bottom: 2px solid #007bff; padding-bottom: 5px; }}
        .status {{ font-size: 24px; font-weight: bold; color: #28a745; margin-bottom: 20px; }}
        .summary {{ background-color: #f8f9fa; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #007bff; color: white; font-weight: bold; }}
        tr:hover {{ background-color: #f5f5f5; }}
        .footer {{ margin-top: 20px; color: #666; font-size: 12px; text-align: center; }}
        .analytics-table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
        .analytics-table th {{ background-color: #6c757d; color: white; font-weight: bold; padding: 8px; text-align: center; }}
        .analytics-table td {{ padding: 4px; text-align: center; vertical-align: middle; border: 1px solid #ddd; position: relative; }}
        .analytics-table td.date-cell {{ font-weight: bold; background-color: #f8f9fa; }}
        .bar-container {{ width: 100%; height: 40px; background-color: #e9ecef; position: relative; }}
        .bar {{ height: 100%; background: linear-gradient(to right, #007bff, #0056b3); display: flex; align-items: center; justify-content: center; color: white; font-weight: bold; font-size: 11px; }}
        .bar-archived {{ height: 100%; background: linear-gradient(to right, #dc3545, #bd2130); display: flex; align-items: center; justify-content: center; color: white; font-weight: bold; font-size: 11px; }}
        .bar-empty {{ height: 100%; display: flex; align-items: center; justify-content: center; color: #6c757d; font-size: 11px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>SOAR Archive Report - {}</h1>
        <div class="status">✓ SUCCESS</div>
        <div class="summary">
            {}
        </div>

        <h2>Archive Summary</h2>
        <table>
            <tr>
                <th>Table</th>
                <th>Rows Deleted</th>
                <th>File Size</th>
                <th>Archive Path</th>
                <th>Duration</th>
                <th>Oldest Remaining</th>
                <th>Relative</th>
            </tr>"#,
            environment, summary_html
        );

        for table in &self.tables {
            let oldest_str = table
                .oldest_remaining
                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let relative_str = table
                .oldest_remaining
                .map(Self::relative_time_days)
                .unwrap_or_else(|| "N/A".to_string());

            html.push_str(&format!(
                r#"
            <tr>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td style="font-family: monospace; font-size: 11px;">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>"#,
                table.table_name,
                Self::format_number(table.rows_deleted),
                Self::format_file_size(table.file_size_bytes),
                table.file_path,
                Self::format_duration(table.duration_secs),
                oldest_str,
                relative_str
            ));
        }

        html.push_str("</table>");

        // Add analytics section
        if !self.daily_counts.is_empty() {
            html.push_str(
                r#"
        <h2>Analytics</h2>
        <table class="analytics-table">
            <tr>
                <th>Date</th>"#,
            );

            // Add column headers for each table
            for table in &self.tables {
                html.push_str(&format!("<th>{}</th>", table.table_name));
            }
            html.push_str("</tr>");

            // Find all unique dates across all tables and get the max count per table
            let mut all_dates = std::collections::HashSet::new();
            let mut max_counts: HashMap<String, i64> = HashMap::new();

            for (table_name, counts) in &self.daily_counts {
                for daily_count in counts {
                    all_dates.insert(daily_count.date);
                    let max = max_counts.entry(table_name.clone()).or_insert(0);
                    if daily_count.count > *max {
                        *max = daily_count.count;
                    }
                }
            }

            // Convert to sorted vec (oldest first)
            let mut dates: Vec<NaiveDate> = all_dates.into_iter().collect();
            dates.sort();

            // Create rows for each date
            for date in dates {
                html.push_str(&format!(
                    r#"
            <tr>
                <td class="date-cell">{}</td>"#,
                    date.format("%Y-%m-%d")
                ));

                // Add cell for each table
                for table in &self.tables {
                    if let Some(counts) = self.daily_counts.get(&table.table_name) {
                        if let Some(daily_count) = counts.iter().find(|dc| dc.date == date) {
                            let max_count = max_counts.get(&table.table_name).copied().unwrap_or(1);
                            let percentage = if max_count > 0 {
                                (daily_count.count as f64 / max_count as f64 * 100.0) as u32
                            } else {
                                0
                            };

                            // Format count in thousands (K)
                            let count_display = if daily_count.count >= 1000 {
                                format!("{} K", Self::format_count(daily_count.count / 1000))
                            } else {
                                Self::format_count(daily_count.count)
                            };

                            let bar_class = if daily_count.archived {
                                "bar-archived"
                            } else {
                                "bar"
                            };
                            html.push_str(&format!(
                                r#"
                <td>
                    <div class="bar-container">
                        <div class="{}" style="width: {}%;">{}</div>
                    </div>
                </td>"#,
                                bar_class, percentage, count_display
                            ));
                        } else {
                            html.push_str(
                                r#"
                <td>
                    <div class="bar-container">
                        <div class="bar-empty">0</div>
                    </div>
                </td>"#,
                            );
                        }
                    } else {
                        html.push_str(
                            r#"
                <td>
                    <div class="bar-container">
                        <div class="bar-empty">-</div>
                    </div>
                </td>"#,
                        );
                    }
                }

                html.push_str("</tr>");
            }

            html.push_str("</table>");
        }

        html.push_str(
            r#"
        <div class="footer">
            Generated by SOAR Archive System
        </div>
    </div>
</body>
</html>"#,
        );

        html
    }
}

pub fn send_archive_email_report(
    config: &crate::email_reporter::EmailConfig,
    report: &ArchiveReport,
) -> Result<()> {
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};
    use std::time::Duration;
    use tracing::info;

    let staging_prefix = get_staging_prefix();
    let subject = format!(
        "{}✓ SOAR Archive Complete - {}",
        staging_prefix,
        chrono::Local::now().format("%Y-%m-%d")
    );

    let html_body = report.to_html();

    info!("Sending archive email report to {}", config.to_address);

    let email = Message::builder()
        .from(config.from_address.parse()?)
        .to(config.to_address.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body)?;

    let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

    let mailer = SmtpTransport::relay(&config.smtp_server)?
        .port(config.smtp_port)
        .credentials(creds)
        .timeout(Some(Duration::from_secs(30)))
        .build();

    match mailer.send(&email) {
        Ok(_) => {
            info!("Archive email report sent successfully");
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Failed to send archive email report: {}", e);
            Err(anyhow::anyhow!("Failed to send email: {}", e))
        }
    }
}

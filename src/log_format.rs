//! Custom log format that displays target before span context.
//!
//! Default tracing format: `LEVEL span1:span2: target: message`
//! This format:            `LEVEL target: span1:span2: message`

use std::fmt;
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::format::{self, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// Custom event formatter that puts target before span context
pub struct TargetFirstFormat;

impl<S, N> FormatEvent<S, N> for TargetFirstFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();

        // Write level with color
        let level = metadata.level();
        let level_style = match *level {
            tracing::Level::ERROR => "\x1b[31m", // Red
            tracing::Level::WARN => "\x1b[33m",  // Yellow
            tracing::Level::INFO => "\x1b[32m",  // Green
            tracing::Level::DEBUG => "\x1b[34m", // Blue
            tracing::Level::TRACE => "\x1b[35m", // Magenta
        };
        write!(writer, "{}{:>5}\x1b[0m ", level_style, level)?;

        // Write target (module path)
        write!(writer, "{}: ", metadata.target())?;

        // Write span context (if any)
        if let Some(scope) = ctx.event_scope() {
            let mut first = true;
            for span in scope.from_root() {
                if !first {
                    write!(writer, ":")?;
                }
                write!(writer, "{}", span.name())?;
                first = false;
            }
            if !first {
                write!(writer, ": ")?;
            }
        }

        // Write the event message and fields
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

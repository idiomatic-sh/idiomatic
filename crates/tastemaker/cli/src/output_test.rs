//! Proves how a command's answer is printed in each format, and each error with its next step.

use super::{Format, FormatSwitch, markdown, print, said, unwritten};
use crate::outcome::Outcome;

/// An answer of one line of text, as a command's `schemas.rs` would write it.
struct Line(&'static str);

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl super::Encoded for Line {
    fn json(&self, to: &mut dyn std::io::Write, spacing: super::Spacing) -> std::io::Result<()> {
        match spacing {
            super::Spacing::Compact => serde_json::to_writer(to, self.0),
            super::Spacing::Indented => serde_json::to_writer_pretty(to, self.0),
        }
        .map_err(std::io::Error::from)
    }
}

/// A switch with the three flags a command line can give.
fn switch(format: Format, json: bool, quiet: bool) -> FormatSwitch {
    FormatSwitch {
        format,
        json,
        quiet,
    }
}

/// Without a flag the format named by `--format` is the one chosen.
#[test]
fn format_succeeds_as_the_one_named() {
    assert_eq!(
        switch(Format::Markdown, false, false).format(),
        Format::Markdown
    );
    assert_eq!(
        switch(Format::Terminal, false, false).format(),
        Format::Terminal
    );
}

/// `--json` is `--format json`.
#[test]
fn json_succeeds_as_the_json_format() {
    assert_eq!(switch(Format::Terminal, true, false).format(), Format::Json);
}

/// `--quiet` wins over every other flag.
#[test]
fn quiet_succeeds_over_every_other_flag() {
    assert_eq!(switch(Format::Markdown, true, true).format(), Format::Quiet);
}

/// A quiet answer prints nothing.
#[test]
fn print_succeeds_quietly() {
    print(Format::Quiet, &Line("an answer")).unwrap();
}

/// Markdown is the answer's text with its styles dropped.
#[test]
fn markdown_succeeds_without_styles() {
    assert_eq!(markdown(&"\u{1b}[1mbold\u{1b}[0m text"), "bold text");
}

/// An answer stdout refused ends the command as an error.
#[test]
fn unwritten_refuses_as_an_error() {
    assert_eq!(
        unwritten(&std::io::Error::other("the disk is full")),
        Outcome::Error
    );
}

/// A cause its error's text already says is not said again.
#[test]
fn a_cause_succeeds_said_once() {
    let error = tastemaker_core::inbound::use_cases::scaffold::errors::ScaffoldError::Render {
        file: "a.jinja".into(),
        source: tastemaker_core::outbound::templates::errors::TemplatesError::Render {
            reason: "undefined value".into(),
        },
    };
    assert_eq!(
        said(&error),
        "`a.jinja`: the template does not render: undefined value"
    );
}

/// A buffer the warnings layer writes into, which the test reads back.
#[derive(Clone, Default)]
struct Shared(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A warning the core gives is said as one `warning:` line; nothing else the core says reaches the writer.
#[test]
fn warning_succeeds_as_one_line() {
    use tracing_subscriber::layer::SubscriberExt as _;
    let shared = Shared::default();
    let layer = super::Warnings {
        out: std::sync::Mutex::new(shared.clone()),
    };
    let subscriber = tracing_subscriber::registry().with(tracing_subscriber::Layer::with_filter(
        layer,
        tracing_subscriber::filter::Targets::new()
            .with_target("tastemaker_core", tracing::Level::WARN),
    ));
    tracing::subscriber::with_default(subscriber, || {
        tracing::warn!(target: "tastemaker_core", message = %"cache not saved in `/tmp/x`: denied");
        tracing::info!(target: "tastemaker_core", "not a warning");
        tracing::warn!(target: "elsewhere", "not the core's");
    });
    let written = String::from_utf8(
        shared
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone(),
    )
    .unwrap_or_default();
    assert_eq!(written, "warning: cache not saved in `/tmp/x`: denied\n");
}

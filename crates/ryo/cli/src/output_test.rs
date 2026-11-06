//! Proves how a command's answer is printed in each format, and each error with its next step.

use super::{Format, FormatSwitch, markdown, print, unwritten, written};
use crate::outcome::CommandOutcome;

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
        CommandOutcome::Error
    );
}

/// A stream whose every write ends in `kind`: a reader that closed, or a disk that is full.
struct Refusing(std::io::ErrorKind);

impl std::io::Write for Refusing {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from(self.0))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::from(self.0))
    }
}

/// Writes a whole JSON answer onto a stream that refuses every write with `kind`, and returns the kind of the refusal `written` hands on.
fn json_onto_refusing(kind: std::io::ErrorKind) -> Result<(), std::io::ErrorKind> {
    written(
        Format::Json,
        Box::new(Refusing(kind)),
        |mut held| std::io::Write::write_all(&mut held, b"{}").map_err(|error| error.kind()),
        std::io::Error::kind,
    )
}

/// A JSON answer is held until the run ends well, so the refusal of the stream is the one that comes out.
#[test]
fn written_hands_on_the_refusal_of_a_closed_stream() {
    assert_eq!(
        json_onto_refusing(std::io::ErrorKind::BrokenPipe),
        Err(std::io::ErrorKind::BrokenPipe)
    );
}

/// A write that fails for any other reason is handed on as that reason.
#[test]
fn written_hands_on_the_refusal_of_a_full_disk() {
    assert_eq!(
        json_onto_refusing(std::io::ErrorKind::StorageFull),
        Err(std::io::ErrorKind::StorageFull)
    );
}

/// A run that stops leaves the stream without a prefix of a document: `body`'s failure comes out and nothing was written.
#[test]
fn written_leaves_nothing_when_the_body_stops() {
    let outcome: Result<(), &str> = written(
        Format::Json,
        Box::new(Refusing(std::io::ErrorKind::StorageFull)),
        |_held| Err("stopped"),
        |_error| "unwritten",
    );
    assert_eq!(outcome, Err("stopped"));
}

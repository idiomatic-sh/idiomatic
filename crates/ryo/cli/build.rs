//! Writes `src/languages.rs`, the one file that links the language crates, from the directories of
//! `languages/`: one link for each, behind the Cargo feature of its name. Cargo forces the feature
//! and its optional dependency to be written in `Cargo.toml`; this script refuses the build when a
//! language directory has none, so no language is linked by a list kept by hand and none is
//! forgotten. The file is kept in the repository as it is written, and rewritten only when it changes; the text is written as rustfmt lays it out (no call chain whose width depends on a language's name), so a build never dirties the committed file. A language in the workspace's `exclude` is not yet written and is not linked.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

/// The directory of the language crates, from this crate's.
const LANGUAGES: &str = "../languages";

/// The head of the file, which the `languages` template of the command line spec gives it.
const HEAD: &str = "// this file is the command line's one registry of languages, generated from the directories of `languages/`: it is the edge's composition of them, so it hands the language factory the parts of each language and calls it, as a composition.rs would.
//! The language providers this build links, one per enabled feature: each is linked for its
//! registration, which discovery finds, and is named nowhere else.

use ryo_core::outbound::corpus::types::SkippedDirectories;
use ryo_core::outbound::language::{interface::LanguagePort, factory::LanguageConnection};

/// The handles of one linked language: what the language factory is given of it.
macro_rules! handles {
    ($language:expr) => {{
        let language = $language;
        LanguageConnection::from(language)
    }};
}

/// The directories one linked language never reads.
macro_rules! skips {
    ($language:expr) => {
        $language.skips().iter().map(|skip| skip.0.into())
    };
}

/// The languages this build links, in the order of the directories of `languages/`: one for each enabled feature.
#[must_use]
pub(crate) fn linked() -> impl LanguagePort {
    let mut languages = Vec::new();
";

/// The end of `linked`, and the head of `skipped`.
const TAIL: &str = "    ryo_core::outbound::language::factory::use_case(languages)
}

/// The directories the languages this build links never read, at any depth: a walk of the corpus does not go into them.
#[must_use]
pub(crate) fn skipped() -> SkippedDirectories {
    let mut skipped = Vec::<Box<str>>::new();
";

/// The end of the file.
const END: &str = "    SkippedDirectories(skipped)
}
";

pub(crate) fn main() -> Result<(), String> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Cargo archives carry the generated registry, but not sibling provider sources.
    // Keep directory discovery mandatory in a checkout; only Cargo's normalized package
    // can use the already generated file without rewriting packaged sources.
    let listing = std::fs::read_dir(crate_dir.join(LANGUAGES));
    let packaged = listing
        .as_ref()
        .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound);
    if packaged && crate_dir.join("Cargo.toml.orig").is_file() {
        // Only that the generated file is there matters, not what the file system says of it.
        let _: std::fs::Metadata = std::fs::metadata(crate_dir.join("src/languages.rs"))
            .map_err(|error| error.to_string())?;
        return writeln!(
            std::io::stdout().lock(),
            "cargo::rerun-if-changed=src/languages.rs"
        )
        .map_err(|error| error.to_string());
    }
    let manifest =
        std::fs::read_to_string(crate_dir.join("Cargo.toml")).map_err(|error| error.to_string())?;
    let mut names: Vec<String> = listing
        .map_err(|error| error.to_string())?
        .flatten()
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut text = HEAD.to_owned();
    for name in &names {
        if !manifest
            .lines()
            .any(|line| line.starts_with(&format!("{name} = [")))
        {
            return Err(format!(
                "`languages/{name}` has no Cargo feature `{name}` in cli/Cargo.toml: add `{name} = [\"dep:ryo_{name}\"]` and the optional dependency `ryo_{name}`"
            ));
        }
        write!(
            text,
            "    #[cfg(feature = \"{name}\")]\n    languages.push(handles!(ryo_{name}::linked()));\n"
        )
        .map_err(|error| error.to_string())?;
    }
    text.push_str(TAIL);
    for name in &names {
        write!(
            text,
            "    #[cfg(feature = \"{name}\")]\n    skipped.extend(skips!(ryo_{name}::linked()));\n"
        )
        .map_err(|error| error.to_string())?;
    }
    text.push_str(END);
    let file = crate_dir.join("src/languages.rs");
    let written = match std::fs::read_to_string(&file) {
        Ok(written) => Some(written),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    if written.as_deref() != Some(text.as_str()) {
        std::fs::write(&file, text).map_err(|error| error.to_string())?;
    }
    let mut out = std::io::stdout().lock();
    writeln!(out, "cargo::rerun-if-changed={LANGUAGES}").map_err(|error| error.to_string())?;
    writeln!(out, "cargo::rerun-if-changed=../Cargo.toml").map_err(|error| error.to_string())?;
    writeln!(out, "cargo::rerun-if-changed=Cargo.toml").map_err(|error| error.to_string())
}

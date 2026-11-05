// this file is the command line's one registry of languages, generated from the directories of `languages/`: it is the edge's composition of them, so it hands the language factory the parts of each language and calls it, as a composition.rs would.
//! The language providers this build links, one per enabled feature: each is linked for its
//! registration, which discovery finds, and is named nowhere else.

use ryo_core::outbound::corpus::types::SkippedDirectories;
use ryo_core::outbound::language::{interface::LanguagePort, types::LanguageHandles};

/// The handles of one linked language: what the language factory is given of it.
macro_rules! handles {
    ($language:expr) => {{
        let language = $language;
        LanguageHandles::from(language)
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
    #[cfg(feature = "css")]
    languages.push(handles!(ryo_css::linked()));
    #[cfg(feature = "go")]
    languages.push(handles!(ryo_go::linked()));
    #[cfg(feature = "html")]
    languages.push(handles!(ryo_html::linked()));
    #[cfg(feature = "javascript")]
    languages.push(handles!(ryo_javascript::linked()));
    #[cfg(feature = "json")]
    languages.push(handles!(ryo_json::linked()));
    #[cfg(feature = "markdown")]
    languages.push(handles!(ryo_markdown::linked()));
    #[cfg(feature = "python")]
    languages.push(handles!(ryo_python::linked()));
    #[cfg(feature = "rust")]
    languages.push(handles!(ryo_rust::linked()));
    #[cfg(feature = "toml")]
    languages.push(handles!(ryo_toml::linked()));
    #[cfg(feature = "typescript")]
    languages.push(handles!(ryo_typescript::linked()));
    ryo_core::outbound::language::factory::use_case(languages)
}

/// The directories the languages this build links never read, at any depth: a walk of the corpus does not go into them.
#[must_use]
pub(crate) fn skipped() -> SkippedDirectories {
    let mut skipped = Vec::<Box<str>>::new();
    #[cfg(feature = "css")]
    skipped.extend(skips!(ryo_css::linked()));
    #[cfg(feature = "go")]
    skipped.extend(skips!(ryo_go::linked()));
    #[cfg(feature = "html")]
    skipped.extend(skips!(ryo_html::linked()));
    #[cfg(feature = "javascript")]
    skipped.extend(skips!(ryo_javascript::linked()));
    #[cfg(feature = "json")]
    skipped.extend(skips!(ryo_json::linked()));
    #[cfg(feature = "markdown")]
    skipped.extend(skips!(ryo_markdown::linked()));
    #[cfg(feature = "python")]
    skipped.extend(skips!(ryo_python::linked()));
    #[cfg(feature = "rust")]
    skipped.extend(skips!(ryo_rust::linked()));
    #[cfg(feature = "toml")]
    skipped.extend(skips!(ryo_toml::linked()));
    #[cfg(feature = "typescript")]
    skipped.extend(skips!(ryo_typescript::linked()));
    SkippedDirectories(skipped)
}

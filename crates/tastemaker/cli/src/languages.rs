//! The languages this build links, one for each enabled feature: the edge is the one place that names them, and hands
//! them to the analysis port's factory.

/// The languages this build links, in name order.
#[must_use]
pub(crate) fn linked() -> ryo::linter::Languages {
    ryo::linter::Languages(vec![
        #[cfg(feature = "css")]
        ryo_css::linked(),
        #[cfg(feature = "go")]
        ryo_go::linked(),
        #[cfg(feature = "html")]
        ryo_html::linked(),
        #[cfg(feature = "javascript")]
        ryo_javascript::linked(),
        #[cfg(feature = "json")]
        ryo_json::linked(),
        #[cfg(feature = "markdown")]
        ryo_markdown::linked(),
        #[cfg(feature = "python")]
        ryo_python::linked(),
        #[cfg(feature = "rust")]
        ryo_rust::linked(),
        #[cfg(feature = "toml")]
        ryo_toml::linked(),
        #[cfg(feature = "typescript")]
        ryo_typescript::linked(),
    ])
}

/// The directories the languages this build links never read, each with the languages that declare it: what every walk of the
/// repository skips beside `.gitignore`.
#[must_use]
pub(crate) fn skipped() -> Vec<tastemaker_core::domain::files::types::SkippedDirectory> {
    linked()
        .skipped()
        .into_iter()
        .map(
            |(name, by)| tastemaker_core::domain::files::types::SkippedDirectory {
                name: tastemaker_core::domain::files::types::DirectoryName(name.0.into()),
                by: by
                    .into_iter()
                    .map(|language| {
                        tastemaker_core::domain::files::types::Declarer(language.0.clone())
                    })
                    .collect(),
            },
        )
        .collect()
}

/// What `-V` says of the languages linked, each as `<language> <version>`, or that none is.
#[must_use]
pub(crate) fn described() -> String {
    let linked = linked();
    if linked.0.is_empty() {
        return "no language linked".to_owned();
    }
    linked
        .0
        .iter()
        .map(|language| format!("{} {}", language.name().0, language.build().0))
        .collect::<Vec<_>>()
        .join(", ")
}

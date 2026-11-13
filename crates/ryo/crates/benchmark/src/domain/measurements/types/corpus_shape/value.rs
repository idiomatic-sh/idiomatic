//! The corpus a language's benchmark measures: that language's projects, two or three of them.
use super::super::super::constants::{FEWEST_PROJECTS, MOST_PROJECTS};
use super::super::LanguageTitle;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Suite;
use ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage;

/// The language a benchmark's corpus must be of, and the name its refusal gives it.
#[derive(Clone, Copy, Debug)]
pub struct CorpusShape<'a> {
    /// The language the staged corpus declares.
    pub language: &'a SuiteLanguage,
    /// The language as the refusal names it.
    pub display: LanguageTitle<'a>,
}

impl CorpusShape<'_> {
    /// Require the staged corpus to be the language's, of two or three projects.
    ///
    /// # Errors
    /// Refuses another language's corpus, or one of fewer or more projects, as `<display> suite requires 2–3 projects`.
    ///
    /// # Examples
    /// ```
    /// use ryo_benchmark::domain::measurements::types::{corpus_shape::value::CorpusShape, LanguageTitle};
    /// use ryo_benchmark_contracts::domain::workloads::types::Suite;
    /// let fixture = "fixture".into();
    /// let shape = CorpusShape { language: &fixture, display: LanguageTitle("Fixture") };
    /// let empty = Suite { language: "fixture".into(), projects: Vec::new() };
    /// assert_eq!(shape.admit(&empty).unwrap_err().to_string(), "Fixture suite requires 2–3 projects");
    /// ```
    pub fn admit(&self, suite: &Suite) -> Result<(), Error> {
        if suite.language != *self.language
            || !(FEWEST_PROJECTS..=MOST_PROJECTS).contains(&suite.projects.len())
        {
            return Err(Error::Invalid(format!(
                "{} suite requires {FEWEST_PROJECTS}–{MOST_PROJECTS} projects",
                self.display.0
            )));
        }
        Ok(())
    }
}

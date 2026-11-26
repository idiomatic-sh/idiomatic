//! Parse native source, report frontend diagnostics, and schedule embedded expression writing.






use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::PythonParserSettings;




use ryo::languages::{self};


/// The `ruff_python_parser` syntax adapter.
#[derive(Debug, Default)]
pub(crate) struct SyntaxRuffPythonParserAdapter {
    /// The settings the factory injects: the target version the parser reads under.
    pub(crate) settings: PythonParserSettings,
    /// Actual whole-file parses, by `read` and by a semantic session, observed per adapter only in tests.
    #[cfg(test)]
    pub(crate) file_parses: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl SyntaxPort for SyntaxRuffPythonParserAdapter {
    fn walk(
        &self,
        text: SourceText<'_>,
        source_kind: PythonSourceKind,
        reported: crate::outbound::syntax::types::ReportedSyntax,
        builder: &mut languages::NodeTableBuilder<'_>,
        names: crate::domain::model::names::types::WalkNames,
        read: &mut dyn FnMut(
            crate::domain::node::types::Walked<'_, '_, '_>,
        ) -> Vec<languages::Position>,
    ) {
        #[cfg(test)]
        let _previous = self
            .file_parses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        super::walk::walked(&self.settings, text, source_kind, reported, builder, names, read);
    }
}

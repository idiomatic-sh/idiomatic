//! The test view of the semantic model: a file parsed and walked by the `ruff_python_parser` adapter's fixtures, with the model
//! built on the first ask, as the production per-file view builds it.

use crate::domain::model::view::service::built;
use crate::domain::model::view::types::{Built, ModelPart, ModelRead, ModelView, SemanticModel};
use crate::settings::PythonParserSettings;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{Count, FileId, NodeTable};

/// The buffers a test keeps between files: the node table and the model.
#[derive(Debug, Default)]
pub(crate) struct ModelFixture {
    /// The node table of the file in hand.
    pub table: NodeTable,
    /// The model, refilled per file.
    pub model: SemanticModel,
    /// The settings the files are parsed under.
    pub settings: PythonParserSettings,
}

/// The per-file view of one parsed file.
#[derive(Debug)]
pub(crate) struct FixtureView<'f> {
    /// The node table.
    pub table: &'f NodeTable,
    /// The library's node of each node id.
    pub(crate) nodes: &'f [AnyNodeRef<'f>],
    /// The file's number.
    pub file: FileId,
    /// The model.
    pub model: &'f mut SemanticModel,
    /// Which parts of the model this file has built.
    pub built: Built,
    /// How many times a filler asked for the model.
    pub asked: Count,
}

impl ModelView for FixtureView<'_> {
    fn semantics(&mut self, part: ModelPart) -> ModelRead<'_> {
        self.asked = Count(self.asked.0.saturating_add(1));
        built(self.model, &mut self.built, part, self.table, self.nodes);
        ModelRead {
            model: self.model,
            table: self.table,
            file: self.file,
        }
    }
}

/// Parses `source`, walks it into the fixture's table, and lends the file's view to `visit`.
pub(crate) fn viewed(
    fixture: &mut ModelFixture,
    source: &str,
    file: FileId,
    visit: &mut dyn FnMut(&mut FixtureView<'_>),
) {
    let model = &mut fixture.model;
    crate::outbound::syntax::adapters::ruff_python_parser::fixtures::model_walk::walked(
        &mut fixture.table,
        source,
        fixture.settings,
        &mut |table, nodes, target| {
            model.target = target;
            visit(&mut FixtureView {
                table,
                nodes,
                file,
                model,
                built: Built::default(),
                asked: Count(0),
            });
        },
    );
}

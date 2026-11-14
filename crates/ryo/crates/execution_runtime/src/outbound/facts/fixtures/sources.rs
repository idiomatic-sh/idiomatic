//! A source of a file's rows the facts contract reads through a lookup: it counts what it is asked for and can refuse.

use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::types::fact_sources::value::{FactSource, TreeSource};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::collections::relations::comment::item::CommentRow;
use ryo_relations::domain::relations::collections::relations::function::item::FunctionRow;
use ryo_relations::domain::relations::types::{
    ColumnsView, FileId, Flag, NodeId, Offset, Relation, RowWriter, Span,
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::stored_facts::types::ReadInPart;

/// A source that writes one row of each relation it is asked for, `function` and `comment` alone, and answers what it was told to say.
#[derive(Debug, Default)]
pub struct SourceFake {
    /// The relations it serves.
    pub serves: Vec<Relation>,
    /// The tree it holds, empty.
    pub tree: Elements,
    /// What it was asked for, one entry per read.
    pub reads: Vec<Vec<Relation>>,
    /// What it says is read in part, on every read.
    pub said: Vec<ReadInPart>,
    /// The relation whose rows it refuses to admit, if any.
    pub refuses: Option<Relation>,
    /// The relations it was asked to admit, in order.
    pub admitted: Vec<Relation>,
}

impl SourceFake {
    /// A source that serves `serves` and says nothing is read in part.
    #[must_use]
    pub fn serving(serves: &[Relation]) -> Self {
        Self {
            serves: serves.to_vec(),
            ..Self::default()
        }
    }
}

/// The span every row of the fake has.
const SPAN: Span = Span {
    start: Offset(0),
    end: Offset(4),
};

impl FactSource for SourceFake {
    fn serves(&self) -> &[Relation] {
        &self.serves
    }

    fn read(
        &mut self,
        _symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<Vec<ReadInPart>, FactsError> {
        self.reads.push(wanted.to_vec());
        for relation in wanted {
            // The fake writes one row of each relation it knows; any other it serves has no row.
            if *relation == Relation::Function {
                out.write(FunctionRow {
                    id: NodeId(1),
                    name: Symbol(0),
                    file: FileId(0),
                    span: SPAN,
                });
            } else if *relation == Relation::Comment {
                out.write(CommentRow {
                    id: NodeId(2),
                    text: Symbol(1),
                    file: FileId(0),
                    span: SPAN,
                    doc: Flag::Lowered,
                });
            }
        }
        Ok(self.said.clone())
    }

    fn admit(&mut self, relation: Relation, _rows: &ColumnsView<'_>) -> Result<(), FactsError> {
        self.admitted.push(relation);
        if self.refuses == Some(relation) {
            Err(FactsError::Refused)
        } else {
            Ok(())
        }
    }
}

impl TreeSource for SourceFake {
    fn tree(&self) -> &Elements {
        &self.tree
    }
}

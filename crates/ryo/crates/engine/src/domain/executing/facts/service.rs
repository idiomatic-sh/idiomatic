//! The services of the `facts` part: today's fill store read as the columns of `FactColumns`.

use super::types::FactColumns;
use crate::domain::code::types::{RelationNumber, Short};
use crate::domain::filled::service::{held_state, whole_rows};
use crate::domain::filled::types::FilledState;
use crate::domain::indexes::reading::service::cell_at;
use crate::domain::indexes::types::RowRun;
use crate::domain::made_tables::reading::service::view_of;
use crate::domain::made_tables::types::{FillStore, ReadSource};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

impl FactColumns for FillStore {
    fn whole(&self, relation: RelationNumber) -> Option<RowRun> {
        (held_state(&self.relations, relation, self.stamp) == FilledState::Whole)
            .then(|| whole_rows(&self.relations, relation, self.stamp))
    }

    fn cell(&self, relation: RelationNumber, column: Short, row: Count) -> CellWord {
        cell_at(view_of(self, ReadSource::Filled(relation), column), row)
    }
}

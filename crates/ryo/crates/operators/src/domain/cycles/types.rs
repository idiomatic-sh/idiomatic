//! The graph of a cycle search over edges, and the room its scratch keeps.
pub use ryo_values::domain::cycles::types::{
    CycleMark, CycleScratch, EdgePlace, EdgeStart, MeetOrder, NodeCount, NodeNumber, WalkStep,
};

/// The graph a cycle search reads: an edge row's source column and destination column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CycleParts {
    /// The source column of an edge row.
    pub(crate) source: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The destination column of an edge row.
    pub(crate) destination: ryo_values::domain::plan_parts::types::ColumnIndex,
}

impl crate::domain::kept_room::types::KeptRoom for CycleScratch {
    fn kept(&mut self) {
        self.order.kept();
        self.low.kept();
        self.open.kept();
        self.found.kept();
        self.stack.kept();
        self.component.kept();
        self.walk.kept();
        self.ordered.kept();
        self.starts.kept();
        self.leaving.kept();
    }

    #[cfg(test)]
    fn room(&self) -> crate::domain::kept_room::types::Room {
        [
            self.order.room(),
            self.low.room(),
            self.open.room(),
            self.found.room(),
            self.stack.room(),
            self.component.room(),
            self.walk.room(),
            self.ordered.room(),
            self.starts.room(),
            self.leaving.room(),
        ]
        .into_iter()
        .max()
        .unwrap_or_default()
    }
}

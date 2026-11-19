//! Columns, stage identities and checked lookup instructions over one file.
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, RepeatedRows, Side, StepId, Test,
};
pub(crate) mod entity_columns;
pub(crate) mod entity_files;
pub(crate) mod entity_plan;
/// The part of an entity that a query reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum EntityAttribute {
    Id,
    Kind,
    Text,
    File,
    Span,
    Ordinal,
    Role,
}
/// Number of immutable parent links to follow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct AncestorDistance(pub usize);
/// An original field and the parent at which its value resides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct EntityColumn(pub EntityAttribute, pub AncestorDistance);
/// A node in the original physical plan, before the entity stages are attached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EntityExportId(pub u32);
/// An original physical result and its checked entity stage.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EntityExport(pub EntityExportId, pub StepId);
/// Literal spelling number in one prepared query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct EntityLiteralCode(pub u32);
/// Mask of immutable source relations needed by a stage or held by a file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EntitySources(pub u8);
/// Local number of an entity within its file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct EntityPosition(pub u32);
/// A branch or rule can ask whether a live file belongs to this query plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityOwnership {
    Current,
    Foreign,
}
/// Count of columns held or scored by the prepared query.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EntityColumnCount(pub usize);
/// A compiled text literal or comparison column.
#[derive(Clone, Debug)]
pub(crate) enum EntityOperand {
    Text(LiteralText, EntityLiteralCode),
    Column(ColumnIndex),
}
/// One total equality or inequality against a column.
#[derive(Clone, Debug)]
pub(crate) struct EntityPredicate {
    pub column: ColumnIndex,
    pub test: Test,
    pub operand: EntityOperand,
}
/// Operations whose results can be represented by ordered entity positions.
#[derive(Clone, Debug)]
pub(crate) enum EntityQueryOp {
    Syntax,
    Edge,
    Project {
        distinct: RepeatedRows,
    },
    Filter {
        tests: Vec<EntityPredicate>,
    },
    Union,
    Anti {
        l: Vec<ColumnIndex>,
        r: Vec<ColumnIndex>,
    },
    Join {
        side: Side,
        depth: AncestorDistance,
        l: Vec<ColumnIndex>,
        r: Vec<ColumnIndex>,
    },
}
/// One checked entity stage and the inputs it reads in ordinary order.
#[derive(Clone, Debug)]
pub(crate) struct EntityStage {
    pub expr: EntityQueryOp,
    pub fields: Vec<EntityColumn>,
    pub operands: Vec<StepId>,
    pub physical: Option<EntityExportId>,
}

pub(crate) mod entity_subscriptions;

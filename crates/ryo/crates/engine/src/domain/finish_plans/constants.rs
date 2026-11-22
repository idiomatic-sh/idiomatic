//! The constants of the `finish_plans` concept: how the plan format writes each step kind, argument, direction, bound
//! and side. The lowering and the encoding read them from here, so the plan and its text agree.

/// How the format writes each step kind.
pub(crate) const OP_UNION: &str = "union";
pub(crate) const OP_ANTIJOIN: &str = "antijoin";
pub(crate) const OP_AS_OF: &str = "as-of";
pub(crate) const OP_ORDERED_MATCH: &str = "ordered-match";
pub(crate) const OP_AXIS: &str = "axis";
pub(crate) const OP_CLOSURE: &str = "closure";
pub(crate) const OP_FOLLOW: &str = "follow";
pub(crate) const OP_ORDERED_DFS: &str = "ordered-dfs";
pub(crate) const OP_GROUP_COUNT: &str = "group-count";
pub(crate) const OP_GROUP_MIN: &str = "group-min";
pub(crate) const OP_GROUP_MAX: &str = "group-max";
pub(crate) const OP_GROUP_ORDINAL: &str = "group-ordinal";
pub(crate) const OP_PROJECT: &str = "project";
pub(crate) const OP_EXTEND: &str = "extend";
pub(crate) const OP_SPAN_BOUNDS: &str = "span-bounds";
pub(crate) const OP_SPAN_BETWEEN: &str = "span-between";
pub(crate) const OP_REPLACE: &str = "replace";
pub(crate) const OP_SOURCE_MATCHES: &str = "source-matches";
pub(crate) const OP_CAPTURE: &str = "capture";
pub(crate) const OP_FITS_RENDERED: &str = "fits-rendered";
pub(crate) const OP_RENDER_SYNTAX: &str = "render-syntax";
pub(crate) const OP_ROWS: &str = "rows";
pub(crate) const OP_PREFIXES: &str = "prefixes";
pub(crate) const OP_SUBTREE_CLASS: &str = "subtree-class";

/// How the format writes each argument of an operator.
pub(crate) const ARG_FROM: &str = "from";
pub(crate) const ARG_LEFT: &str = "left";
pub(crate) const ARG_RIGHT: &str = "right";
pub(crate) const ARG_EDGES: &str = "edges";
pub(crate) const ARG_DOMAIN: &str = "domain";
pub(crate) const ARG_RELATION: &str = "relation";
pub(crate) const ARG_LEFT_ON: &str = "left-on";
pub(crate) const ARG_RIGHT_ON: &str = "right-on";
pub(crate) const ARG_LEFT_ORDER: &str = "left-order";
pub(crate) const ARG_RIGHT_ORDER: &str = "right-order";
pub(crate) const ARG_BOUND: &str = "bound";
pub(crate) const ARG_OUTPUT: &str = "output";
pub(crate) const ARG_DIRECTION: &str = "direction";
pub(crate) const ARG_SOURCE: &str = "source";
pub(crate) const ARG_DESTINATION: &str = "destination";
pub(crate) const ARG_KEYS: &str = "keys";
pub(crate) const ARG_ROOT: &str = "root";
pub(crate) const ARG_ORDER: &str = "order";
pub(crate) const ARG_EDGE: &str = "edge";
pub(crate) const ARG_COUNT: &str = "count";
pub(crate) const ARG_DISTINCT: &str = "distinct";
pub(crate) const ARG_COLUMNS: &str = "columns";
pub(crate) const ARG_VALUES: &str = "values";
pub(crate) const ARG_SPAN: &str = "span";
pub(crate) const ARG_START_OUTPUT: &str = "start-output";
pub(crate) const ARG_END_OUTPUT: &str = "end-output";
pub(crate) const ARG_START: &str = "start";
pub(crate) const ARG_END: &str = "end";
pub(crate) const ARG_TEXT: &str = "text";
pub(crate) const ARG_PATTERN: &str = "pattern";
pub(crate) const ARG_REPLACEMENT: &str = "replacement";
pub(crate) const ARG_BASE_LINE: &str = "base-line";
pub(crate) const ARG_LINE_OUTPUT: &str = "line-output";
pub(crate) const ARG_CAPTURE: &str = "capture";
pub(crate) const ARG_CAPTURE_OUTPUT: &str = "capture-output";
pub(crate) const ARG_CAPTURE_TEXT_OUTPUT: &str = "capture-text-output";
pub(crate) const ARG_CAPTURE_LINE_OUTPUT: &str = "capture-line-output";
pub(crate) const ARG_PATH: &str = "path";
pub(crate) const ARG_CAPTURES: &str = "captures";
pub(crate) const ARG_TEXTS: &str = "texts";
pub(crate) const ARG_LITERALS: &str = "literals";
pub(crate) const ARG_MAX_LINE: &str = "max-line";
pub(crate) const ARG_TAB_WIDTH: &str = "tab-width";
pub(crate) const ARG_NODE: &str = "node";
pub(crate) const ARG_CONTEXT: &str = "context";
pub(crate) const ARG_SEPARATOR: &str = "separator";
pub(crate) const ARG_LABELS: &str = "labels";
pub(crate) const ARG_LEAVES: &str = "leaves";
pub(crate) const ARG_LABEL: &str = "label";
pub(crate) const ARG_LEAF_LABEL: &str = "leaf-label";
pub(crate) const ARG_STEP: &str = "step";

/// The arguments that name a relation the operator reads (a step, or a relation of the catalog), but for `rows`, whose
/// `from` is its constant table.
pub(crate) const RELATION_ARGUMENTS: [&str; 8] =
    [ARG_FROM, ARG_LEFT, ARG_RIGHT, ARG_EDGES, ARG_DOMAIN, ARG_RELATION, ARG_LABELS, ARG_LEAVES];

/// How the format writes each axis direction.
pub(crate) const DIRECTION_PARENT: &str = "parent";
pub(crate) const DIRECTION_CHILD: &str = "child";
pub(crate) const DIRECTION_ANCESTOR: &str = "ancestor";
pub(crate) const DIRECTION_DESCENDANT: &str = "descendant";
pub(crate) const DIRECTION_NEAREST_ANCESTOR: &str = "nearest-ancestor";
pub(crate) const DIRECTION_DEEP: &str = "deep";
pub(crate) const DIRECTION_PREVIOUS_SIBLING: &str = "previous-sibling";
pub(crate) const DIRECTION_NEXT_SIBLING: &str = "next-sibling";

/// How the format writes each `as-of` bound.
pub(crate) const BOUND_AT_OR_BEFORE: &str = "at-or-before";
pub(crate) const BOUND_BEFORE: &str = "before";
pub(crate) const BOUND_AT_OR_AFTER: &str = "at-or-after";
pub(crate) const BOUND_AFTER: &str = "after";

/// How the format writes each side an output column is read from.
pub(crate) const SIDE_LEFT: &str = "left";
pub(crate) const SIDE_RIGHT: &str = "right";

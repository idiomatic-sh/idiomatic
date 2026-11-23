//! The constants of the `finish_plans` concept: how the plan format writes each step kind, argument, direction, bound
//! and side. The lowering and the encoding read them from here, so the plan and its text agree.

/// How the format writes each step kind.
pub const OP_UNION: &str = "union";
/// `antijoin`.
pub const OP_ANTIJOIN: &str = "antijoin";
/// `as-of`.
pub const OP_AS_OF: &str = "as-of";
/// `ordered-match`.
pub const OP_ORDERED_MATCH: &str = "ordered-match";
/// `axis`.
pub const OP_AXIS: &str = "axis";
/// `closure`.
pub const OP_CLOSURE: &str = "closure";
/// `follow`.
pub const OP_FOLLOW: &str = "follow";
/// `ordered-dfs`.
pub const OP_ORDERED_DFS: &str = "ordered-dfs";
/// `group-count`.
pub const OP_GROUP_COUNT: &str = "group-count";
/// `group-min`.
pub const OP_GROUP_MIN: &str = "group-min";
/// `group-max`.
pub const OP_GROUP_MAX: &str = "group-max";
/// `group-ordinal`.
pub const OP_GROUP_ORDINAL: &str = "group-ordinal";
/// `project`.
pub const OP_PROJECT: &str = "project";
/// `extend`.
pub const OP_EXTEND: &str = "extend";
/// `span-bounds`.
pub const OP_SPAN_BOUNDS: &str = "span-bounds";
/// `span-between`.
pub const OP_SPAN_BETWEEN: &str = "span-between";
/// `replace`.
pub const OP_REPLACE: &str = "replace";
/// `source-matches`.
pub const OP_SOURCE_MATCHES: &str = "source-matches";
/// `capture`.
pub const OP_CAPTURE: &str = "capture";
/// `fits-rendered`.
pub const OP_FITS_RENDERED: &str = "fits-rendered";
/// `render-syntax`.
pub const OP_RENDER_SYNTAX: &str = "render-syntax";
/// `rows`.
pub const OP_ROWS: &str = "rows";
/// `prefixes`.
pub const OP_PREFIXES: &str = "prefixes";
/// `subtree-class`.
pub const OP_SUBTREE_CLASS: &str = "subtree-class";

/// How the format writes each argument of an operator.
pub const ARG_FROM: &str = "from";
/// `left`.
pub const ARG_LEFT: &str = "left";
/// `right`.
pub const ARG_RIGHT: &str = "right";
/// `edges`.
pub const ARG_EDGES: &str = "edges";
/// `domain`.
pub const ARG_DOMAIN: &str = "domain";
/// `relation`.
pub const ARG_RELATION: &str = "relation";
/// `left-on`.
pub const ARG_LEFT_ON: &str = "left-on";
/// `right-on`.
pub const ARG_RIGHT_ON: &str = "right-on";
/// `left-order`.
pub const ARG_LEFT_ORDER: &str = "left-order";
/// `right-order`.
pub const ARG_RIGHT_ORDER: &str = "right-order";
/// `bound`.
pub const ARG_BOUND: &str = "bound";
/// `output`.
pub const ARG_OUTPUT: &str = "output";
/// `direction`.
pub const ARG_DIRECTION: &str = "direction";
/// `source`.
pub const ARG_SOURCE: &str = "source";
/// `destination`.
pub const ARG_DESTINATION: &str = "destination";
/// `keys`.
pub const ARG_KEYS: &str = "keys";
/// `root`.
pub const ARG_ROOT: &str = "root";
/// `order`.
pub const ARG_ORDER: &str = "order";
/// `edge`.
pub const ARG_EDGE: &str = "edge";
/// `count`.
pub const ARG_COUNT: &str = "count";
/// `distinct`.
pub const ARG_DISTINCT: &str = "distinct";
/// `columns`.
pub const ARG_COLUMNS: &str = "columns";
/// `values`.
pub const ARG_VALUES: &str = "values";
/// `span`.
pub const ARG_SPAN: &str = "span";
/// `start-output`.
pub const ARG_START_OUTPUT: &str = "start-output";
/// `end-output`.
pub const ARG_END_OUTPUT: &str = "end-output";
/// `start`.
pub const ARG_START: &str = "start";
/// `end`.
pub const ARG_END: &str = "end";
/// `text`.
pub const ARG_TEXT: &str = "text";
/// `pattern`.
pub const ARG_PATTERN: &str = "pattern";
/// `replacement`.
pub const ARG_REPLACEMENT: &str = "replacement";
/// `base-line`.
pub const ARG_BASE_LINE: &str = "base-line";
/// `line-output`.
pub const ARG_LINE_OUTPUT: &str = "line-output";
/// `capture`.
pub const ARG_CAPTURE: &str = "capture";
/// `capture-output`.
pub const ARG_CAPTURE_OUTPUT: &str = "capture-output";
/// `capture-text-output`.
pub const ARG_CAPTURE_TEXT_OUTPUT: &str = "capture-text-output";
/// `capture-texts`.
pub const ARG_CAPTURE_TEXTS: &str = "capture-texts";
/// `capture-line-output`.
pub const ARG_CAPTURE_LINE_OUTPUT: &str = "capture-line-output";
/// `path`.
pub const ARG_PATH: &str = "path";
/// `captures`.
pub const ARG_CAPTURES: &str = "captures";
/// `texts`.
pub const ARG_TEXTS: &str = "texts";
/// `literals`.
pub const ARG_LITERALS: &str = "literals";
/// `max-line`.
pub const ARG_MAX_LINE: &str = "max-line";
/// `tab-width`.
pub const ARG_TAB_WIDTH: &str = "tab-width";
/// `node`.
pub const ARG_NODE: &str = "node";
/// `context`.
pub const ARG_CONTEXT: &str = "context";
/// `separator`.
pub const ARG_SEPARATOR: &str = "separator";
/// `labels`.
pub const ARG_LABELS: &str = "labels";
/// `leaves`.
pub const ARG_LEAVES: &str = "leaves";
/// `label`.
pub const ARG_LABEL: &str = "label";
/// `leaf-label`.
pub const ARG_LEAF_LABEL: &str = "leaf-label";
/// `id`.
pub const ARG_ID: &str = "id";
/// `parent`.
pub const ARG_PARENT: &str = "parent";
/// `child`.
pub const ARG_CHILD: &str = "child";
/// `step`.
pub const ARG_STEP: &str = "step";

/// The arguments that name a relation the operator reads (a step, or a relation of the catalog), but for `rows`, whose
/// `from` is its constant table.
pub const RELATION_ARGUMENTS: [&str; 8] =
    [ARG_FROM, ARG_LEFT, ARG_RIGHT, ARG_EDGES, ARG_DOMAIN, ARG_RELATION, ARG_LABELS, ARG_LEAVES];

/// How the format writes each axis direction.
pub const DIRECTION_PARENT: &str = "parent";
/// `child`.
pub const DIRECTION_CHILD: &str = "child";
/// `ancestor`.
pub const DIRECTION_ANCESTOR: &str = "ancestor";
/// `descendant`.
pub const DIRECTION_DESCENDANT: &str = "descendant";
/// `nearest-ancestor`.
pub const DIRECTION_NEAREST_ANCESTOR: &str = "nearest-ancestor";
/// `deep`.
pub const DIRECTION_DEEP: &str = "deep";
/// `previous-sibling`.
pub const DIRECTION_PREVIOUS_SIBLING: &str = "previous-sibling";
/// `next-sibling`.
pub const DIRECTION_NEXT_SIBLING: &str = "next-sibling";

/// How the format writes each `as-of` bound.
pub const BOUND_AT_OR_BEFORE: &str = "at-or-before";
/// `before`.
pub const BOUND_BEFORE: &str = "before";
/// `at-or-after`.
pub const BOUND_AT_OR_AFTER: &str = "at-or-after";
/// `after`.
pub const BOUND_AFTER: &str = "after";

/// How the format writes each side an output column is read from.
pub const SIDE_LEFT: &str = "left";
/// `right`.
pub const SIDE_RIGHT: &str = "right";

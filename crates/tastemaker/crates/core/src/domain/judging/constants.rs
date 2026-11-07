//! The words of the `judging` concept: what its findings are written with, the relations tm hosts for the count call,
//! their columns, and the RyoQL queries that judge them, which tm hands ryo's SDK as any caller does and whose rows it
//! reads back.

/// The ending of "directory" for one.
pub const ONE_DIRECTORY: &str = "y";

/// The ending of "directory" for any other number.
pub const DIRECTORIES: &str = "ies";

/// What a template entry's count of files counts, as a message says it.
pub const FILES: &str = "file(s)";

/// What a template entry's count of captured values counts, as a message says it.
pub const VALUES: &str = "captured value(s)";

/// Where a scope at the root of the project counts, as a message says it: the root has no directory name to quote.
pub const AT_ROOT: &str = "at the project's root";

/// The counts and pairs a check left waiting, each with its bounds: `entry`, `scope`, `identity`, `pair`, `found`,
/// `condition`, `min`, `max`, `then-min`, `then-max`, `else-min`, `else-max`.
pub const TEMPLATE_ENTRY: &str = "tastemaker.template-entry";
/// What each condition a count reads answered in its scope: `scope`, `condition`, `holds`.
pub const CONDITION_ANSWER: &str = "tastemaker.condition-answer";

/// A count's or a pair's number in the call.
pub const ENTRY_NUMBER: &str = "entry";
/// The scope whose conditions it reads, by its place among the placement's scopes.
pub const SCOPE_INDEX: &str = "scope";
/// The entry that counts.
pub const IDENTITY: &str = "identity";
/// One for a pair, zero for a count.
pub const PAIR_FLAG: &str = "pair";
/// How many were found.
pub const FOUND: &str = "found";
/// The condition it reads; blank when it reads none.
pub const CONDITION: &str = "condition";
/// The fewest, when it reads no condition.
pub const MIN: &str = "min";
/// The most, when it reads no condition; the largest number when it has none.
pub const MAX: &str = "max";
/// The fewest when the condition holds.
pub const THEN_MIN: &str = "then-min";
/// The most when the condition holds.
pub const THEN_MAX: &str = "then-max";
/// The fewest when it does not.
pub const ELSE_MIN: &str = "else-min";
/// The most when it does not.
pub const ELSE_MAX: &str = "else-max";
/// What the condition answered: one when it holds, zero when it does not, two when its rule could not run.
pub const HOLDS: &str = "holds";
/// Which bounds a refused count was held to: zero its own, one the `then`, two the `else`.
pub const BRANCH: &str = "branch";

/// The `holds` of a condition that holds.
pub const HOLDS_YES: u32 = 1;
/// The `holds` of one that does not.
pub const HOLDS_NO: u32 = 0;
/// The `holds` of one whose rule could not run: the counts that read it are not judged.
pub const HOLDS_UNANSWERED: u32 = 2;
/// The `pair` of a pair.
pub const KIND_PAIR: u32 = 1;
/// The `pair` of a count.
pub const KIND_COUNT: u32 = 0;
/// The `branch` of a count held to its own bounds.
pub const BRANCH_OWN: u32 = 0;
/// The `branch` of one held to its `then`.
pub const BRANCH_THEN: u32 = 1;
/// The `branch` of one held to its `else`.
pub const BRANCH_ELSE: u32 = 2;

/// The id of the query of the counts outside their bounds.
pub const COUNT_RULE: &str = "count-refused";
/// The id of the query of the pairs whose findings stand.
pub const PAIR_RULE: &str = "pair-missing";

/// Each count whose found number is outside the bounds it is held to, and which bounds: its own when it reads no
/// condition, else the `then` bounds when its condition holds and the `else` bounds when it does not. A count whose
/// condition's rule could not run is not judged.
pub const COUNT_QUERY: &str = r#"let entries = select * from "tastemaker.template-entry" where pair = 0;
let answers = select * from "tastemaker.condition-answer";
let plain = select e."entry" as "entry" from entries as e where e."condition" = "" and (e."found" < e."min" or e."found" > e."max");
let own = extend(from=plain, values={"branch": 0});
let asked = select e."entry" as "entry", e."found" as "found", e."then-min" as "then-min", e."then-max" as "then-max", e."else-min" as "else-min", e."else-max" as "else-max", a."holds" as "holds" from entries as e join answers as a on e."scope" = a."scope" and e."condition" = a."condition";
let then-refused = select a."entry" as "entry" from asked as a where a."holds" = 1 and (a."found" < a."then-min" or a."found" > a."then-max");
let then = extend(from=then-refused, values={"branch": 1});
let else-refused = select a."entry" as "entry" from asked as a where a."holds" = 0 and (a."found" < a."else-min" or a."found" > a."else-max");
let otherwise = extend(from=else-refused, values={"branch": 2});
let branched = union(left=then, right=otherwise);
let refused = union(left=own, right=branched);
emit refused subject "entry" key("entry");
"#;

/// Each pair whose count requires any file: its own least when it reads no condition, else the least of the branch its
/// condition's answer picks. A pair whose condition's rule could not run is not reported.
pub const PAIR_QUERY: &str = r#"let pairs = select * from "tastemaker.template-entry" where pair = 1;
let answers = select * from "tastemaker.condition-answer";
let plain = select p."entry" as "entry" from pairs as p where p."condition" = "" and p."min" > 0;
let asked = select p."entry" as "entry", p."then-min" as "then-min", p."else-min" as "else-min", a."holds" as "holds" from pairs as p join answers as a on p."scope" = a."scope" and p."condition" = a."condition";
let then = select a."entry" as "entry" from asked as a where a."holds" = 1 and a."then-min" > 0;
let otherwise = select a."entry" as "entry" from asked as a where a."holds" = 0 and a."else-min" > 0;
let branched = union(left=then, right=otherwise);
let required = union(left=plain, right=branched);
emit required subject "entry" key("entry");
"#;

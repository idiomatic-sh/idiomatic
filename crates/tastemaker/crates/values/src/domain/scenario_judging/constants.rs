//! The words of the `scenario_judging` concept: the relations tm hosts for a scenario's verdict, their columns, and the
//! RyoQL queries that judge them, which tm hands ryo's SDK as any caller does and whose rows it reads back.

/// The findings the scenarios expect: `scenario`, `finding`, `rule`, `path`, `line`.
pub const EXPECTED: &str = "tastemaker.expected";
/// The findings the scenarios' checks found: `scenario`, `finding`, `rule`, `full`, `path`, `line`.
pub const FOUND: &str = "tastemaker.found";
/// The rules the scenarios' `only` names: `scenario`, `place`, `only`.
pub const ONLY: &str = "tastemaker.only";
/// The rule each rule's scenario proves: `scenario`, `rule`.
pub const OWN: &str = "tastemaker.own";
/// The rules and checks the scenarios' checks ran: `scenario`, `rule`.
pub const EVALUATED: &str = "tastemaker.evaluated";

/// The scenario a row is of, by its number in the call.
pub const SCENARIO: &str = "scenario";
/// A finding's number in the call.
pub const FINDING: &str = "finding";
/// The rule, as the verdict compares it.
pub const RULE: &str = "rule";
/// The rule's whole id.
pub const FULL: &str = "full";
/// A finding's path.
pub const FINDING_PATH: &str = "path";
/// The line; zero for the whole file.
pub const LINE: &str = "line";
/// A rule's place in `only`.
pub const PLACE: &str = "place";
/// A rule `only` names.
pub const ONLY_RULE: &str = "only";
/// How many findings the scenario expects.
pub const EXPECTED_COUNT: &str = "expected";
/// How many findings its check found.
pub const FOUND_COUNT: &str = "found";

/// The id of the query of the findings expected and not found.
pub const MISSING_RULE: &str = "scenario-missing";
/// The id of the query of the findings found and not expected.
pub const EXTRA_RULE: &str = "scenario-extra";
/// The id of the query of the rule a scenario proves, never run.
pub const OWN_UNRAN_RULE: &str = "scenario-own-unran";
/// The id of the query of the rules `only` names, never run.
pub const ONLY_UNRAN_RULE: &str = "scenario-only-unran";

/// The rule, path and line of each scenario whose findings it expects more of than its check found, with both counts.
/// The findings it judges are every one its check found when its `only` names none, else those of the rules `only`
/// names, by their whole id or the id after a package's `/` or a rule's `#`.
pub const MISSING_QUERY: &str = r##"let got = select * from "tastemaker.found";
let named = select * from "tastemaker.only";
let picked = select f."scenario" as "scenario", f."finding" as "finding", f."rule" as "rule", f."path" as "path", f."line" as "line" from got as f join named as o on f."scenario" = o."scenario" where f."full" = o."only" or ends_with(f."full", concat("/", o."only")) or ends_with(f."full", concat("#", o."only"));
let unnamed = antijoin(left=got, right=named, left-on=["scenario"], right-on=["scenario"]);
let free = project(from=unnamed, columns=["scenario", "finding", "rule", "path", "line"]);
let kept = union(left=picked, right=free);
let found-count = group-count(from=kept, keys=["scenario", "rule", "path", "line"], count="found", distinct="finding");
let wanted = select * from "tastemaker.expected";
let expected-count = group-count(from=wanted, keys=["scenario", "rule", "path", "line"], count="expected", distinct="finding");
let short = select e."scenario" as "scenario", e."rule" as "rule", e."path" as "path", e."line" as "line", e."expected" as "expected", f."found" as "found" from expected-count as e join found-count as f on e."scenario" = f."scenario" and e."rule" = f."rule" and e."path" = f."path" and e."line" = f."line" where e."expected" > f."found";
let unfound = antijoin(left=expected-count, right=found-count, left-on=["scenario", "rule", "path", "line"], right-on=["scenario", "rule", "path", "line"]);
let none = extend(from=unfound, values={"found": 0});
let missing = union(left=short, right=none);
emit missing subject "scenario" key("scenario", "rule", "path", "line");
"##;

/// The rule, path and line of each scenario whose check found more findings than it expects, with both counts, over
/// the same findings as `MISSING_QUERY` judges.
pub const EXTRA_QUERY: &str = r##"let got = select * from "tastemaker.found";
let named = select * from "tastemaker.only";
let picked = select f."scenario" as "scenario", f."finding" as "finding", f."rule" as "rule", f."path" as "path", f."line" as "line" from got as f join named as o on f."scenario" = o."scenario" where f."full" = o."only" or ends_with(f."full", concat("/", o."only")) or ends_with(f."full", concat("#", o."only"));
let unnamed = antijoin(left=got, right=named, left-on=["scenario"], right-on=["scenario"]);
let free = project(from=unnamed, columns=["scenario", "finding", "rule", "path", "line"]);
let kept = union(left=picked, right=free);
let found-count = group-count(from=kept, keys=["scenario", "rule", "path", "line"], count="found", distinct="finding");
let wanted = select * from "tastemaker.expected";
let expected-count = group-count(from=wanted, keys=["scenario", "rule", "path", "line"], count="expected", distinct="finding");
let over = select f."scenario" as "scenario", f."rule" as "rule", f."path" as "path", f."line" as "line", f."found" as "found", e."expected" as "expected" from found-count as f join expected-count as e on f."scenario" = e."scenario" and f."rule" = e."rule" and f."path" = e."path" and f."line" = e."line" where f."found" > e."expected";
let unexpected = antijoin(left=found-count, right=expected-count, left-on=["scenario", "rule", "path", "line"], right-on=["scenario", "rule", "path", "line"]);
let none = extend(from=unexpected, values={"expected": 0});
let extra = union(left=over, right=none);
emit extra subject "scenario" key("scenario", "rule", "path", "line");
"##;

/// The rule each rule's scenario proves that its check never ran: the rule reached no file of the project.
pub const OWN_UNRAN_QUERY: &str = r#"let proved = select * from "tastemaker.own";
let ran = select * from "tastemaker.evaluated";
let unran = antijoin(left=proved, right=ran, left-on=["scenario", "rule"], right-on=["scenario", "rule"]);
emit unran subject "scenario" key("scenario", "rule");
"#;

/// Each rule a scenario's `only` names that its check did not run, by its whole id or the id after a package's `/` or a
/// rule's `#`.
pub const ONLY_UNRAN_QUERY: &str = r##"let named = select * from "tastemaker.only";
let ran = select * from "tastemaker.evaluated";
let reached = select o."scenario" as "scenario", o."place" as "place", o."only" as "only" from named as o join ran as v on o."scenario" = v."scenario" where v."rule" = o."only" or ends_with(v."rule", concat("/", o."only")) or ends_with(v."rule", concat("#", o."only"));
let unran = antijoin(left=named, right=reached, left-on=["scenario", "place"], right-on=["scenario", "place"]);
emit unran subject "scenario" key("scenario", "place", "only");
"##;

//! The JSON adapter's cases: the passes' small plan (`plan_lowering/fixtures.rs`) written as Python's `json.dump` wrote
//! the same plan (the bytes below are that file, `pass_semi.py`'s input), and those bytes read back as the plan.

use super::adapter::PlanCodecJsonAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::finish_plans::types::FinishPlan;
use crate::domain::plan_lowering::fixtures::small_plan;
use crate::outbound::plan_codec::errors::PlanCodecError;
use crate::outbound::plan_codec::interface::PlanCodecPort as _;

/// The small plan as Python's `json.dump` wrote it.
const PYTHON_BYTES: &str = r#"{"nodes": [{"op": "base", "rel": "a"}, {"op": "base", "rel": "b"}, {"op": "select", "src": [[0, "l"], [1, "r"]], "items": [["id", ["col", "id", "l"]]], "cond": ["cmp", "=", ["col", "id", "l"], ["col", "id", "r"]]}, {"op": "union", "args": {"left": {"rel": 2}, "right": {"rel": 2}}}, {"op": "select", "src": [[0, "l"], [1, "r"]], "items": [["id", ["col", "id", "r"]]], "cond": ["cmp", "=", ["col", "id", "l"], ["col", "id", "r"]]}, {"op": "select", "src": [[0, "l"], [1, "r"]], "items": [["id", ["col", "id", "l"]], ["other", ["col", "id", "r"]]], "cond": ["cmp", "=", ["col", "id", "l"], ["col", "id", "r"]]}, {"op": "select", "src": [[0, "l"], [1, "r"]], "items": [["id", ["col", "id", "l"]]], "cond": ["cmp", "=", ["col", "kind", "l"], ["lit", "x"]]}, {"op": "select", "src": [[0, "l"], [1, "r"]], "items": [["id", ["col", "id", "l"]]], "cond": ["cmp", "=", ["col", "file", "l"], ["col", "file", "r"]]}, {"op": "group-count", "args": {"from": {"rel": 7}, "keys": ["id"], "count": "n"}}, {"op": "select", "src": [[0, "l"], [1, "r"]], "items": [["id", ["col", "id", "l"]]], "cond": ["cmp", "=", ["col", "span", "l"], ["col", "span", "r"]]}, {"op": "group-count", "args": {"from": {"rel": 9}, "keys": ["id"], "count": "n", "distinct": "id"}}, {"op": "axis", "args": {"from": {"rel": 0}, "relation": {"rel": 1}, "direction": "ancestor", "domain": {"rel": 0}}}, {"op": "select", "src": [[11, null]], "items": [["id", ["col", "id", null]]], "cond": null}, {"op": "axis", "args": {"from": {"rel": 1}, "relation": {"rel": 1}, "direction": "descendant", "domain": {"rel": 0}}}, {"op": "select", "src": [[13, null]], "items": [["related", ["col", "related", null]]], "cond": null}, {"op": "axis", "args": {"from": {"rel": 0}, "relation": {"rel": 0}, "direction": "parent", "domain": {"rel": 1}}}], "emits": {"R1": {"node": 3, "subject": "file", "key": ["id"]}, "R2": {"node": 4, "subject": "file", "key": ["id"]}, "R3": {"node": 5, "subject": "file", "key": ["id"]}, "R4": {"node": 6, "subject": "file", "key": ["id"]}, "R5": {"node": 8, "subject": "file", "key": ["id"]}, "R6": {"node": 10, "subject": "file", "key": ["id"]}, "R7": {"node": 12, "subject": "file", "key": ["id"]}, "R8": {"node": 14, "subject": "file", "key": ["related"]}, "R9": {"node": 15, "subject": "file", "key": ["id"]}}, "failed": {}}"#;

type Input = FinishPlan;
/// What differs: the bytes written, when they are not Python's; the plan read back, when it is not the plan written.
type Value = (Option<String>, Option<FinishPlan>);

const CASES: &[Case<Input, Value, PlanCodecError>] = &[Case {
    name: CaseName("a_plan_is_written_as_python_writes_it_and_reads_back_as_itself"),
    word: Word::Succeeds,
    input: small_plan,
    expect: Expect::Value((None, None)),
}];

fn subject(plan: Input) -> Result<Value, PlanCodecError> {
    let bytes = PlanCodecJsonAdapter.encoded(&plan);
    let read = PlanCodecJsonAdapter.decoded(&bytes)?;
    let written = String::from_utf8_lossy(&bytes).into_owned();
    Ok(((written != PYTHON_BYTES).then_some(written), (read != plan).then_some(read)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

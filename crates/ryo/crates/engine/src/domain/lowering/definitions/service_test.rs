//! The cases of the `definitions` part: a definition is made once for its node and its programs do not depend on which rule
//! asked first; a pattern is asked once; a constant text has one number.

use super::service::{asked, defined, text, use_branch};
use super::types::{Drafts, Flattened, RowEstimate};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::described::described;
use crate::domain::lowering::fixtures::named;
use crate::domain::lowering::nodes::types::{NodeHome, StepShape};
use crate::domain::lowering::service::lowered;
use crate::domain::lowering::types::{SelectedRule, SelectedRules};
use crate::domain::statistics::types::Statistics;
use crate::domain::steps::types::{ColumnMask, MadeColumn};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The scenes lowered together, in this order.
type Input = &'static [&'static str];
/// The lines of the pack's definitions and tables, sorted: the rules' own lines left out.
type Value = Vec<String>;
type Failure = ();

fn subject(scenes: Input) -> Result<Value, Failure> {
    let plans: Vec<_> = scenes.iter().map(|scene| named(scene)).collect();
    let rules: Vec<SelectedRule<'_>> = plans
        .iter()
        .zip(0..)
        .map(|(plan, rule)| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan,
            bindings: RuleBindings(&[]),
        })
        .collect();
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
    };
    let lowered = lowered(SelectedRules(&rules), &Statistics::default(), &kinds);
    let mut lines: Vec<String> = described(&lowered.pack)
        .into_iter()
        .filter(|line| line.starts_with("  pattern") || line.starts_with("  pipeline"))
        .collect();
    lines.sort();
    Ok(lines)
}

/// A definition's programs are the definition's: two rules that read one sub-plan give it the same patterns and the same
/// programs whichever is lowered first.
#[test]
fn a_definitions_program_is_not_its_callers() {
    let first = subject(&["the words a", "pairs within one group"]);
    let second = subject(&["pairs within one group", "the words a"]);
    assert!(first.as_ref().is_ok_and(|lines| !lines.is_empty()), "no definition was lowered: {first:?}");
    assert_eq!(first, second);
}

/// The rows of this part's cases: what the drafts count after a sequence of asks.
const CASES: &[Case<(), (usize, usize, usize, usize), Failure>] = &[Case {
    name: CaseName("a_node_has_one_definition_a_pattern_is_asked_once_and_a_text_has_one_number"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((1, 2, 2, 2)),
}];

/// The definitions, the patterns asked of the first, the patterns waiting, and the texts, after one node is defined twice,
/// two patterns are asked of it (one of them twice) and two texts are written (one of them twice).
fn counted((): ()) -> Result<(usize, usize, usize, usize), Failure> {
    let mut drafts = Drafts::default();
    let home = NodeHome {
        rule: 0,
        step: StepId(0),
    };
    let shape = || StepShape(vec![CellKind::Node, CellKind::File]);
    let node = NodeDigest([7; 16]);
    let first = defined(&mut drafts, node, home, shape(), Flattened::default(), RowEstimate(1));
    let again = defined(&mut drafts, node, home, shape(), Flattened::default(), RowEstimate(1));
    assert_eq!(first, again);
    let key = ColumnMask([MadeColumn(0)].into());
    let nothing = ColumnMask::default();
    asked(&mut drafts, first, &key, &nothing);
    asked(&mut drafts, first, &nothing, &nothing);
    asked(&mut drafts, first, &key, &nothing);
    assert_eq!(text(&mut drafts, "a"), text(&mut drafts, "a"));
    assert_ne!(text(&mut drafts, "a"), text(&mut drafts, "b"));
    assert_eq!(use_branch(&mut drafts, first).row.len(), 2);
    let patterns = drafts.definitions.first().map_or(0, |draft| draft.patterns.len());
    Ok((drafts.definitions.len(), patterns, drafts.waiting.len(), drafts.texts.len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, counted);
}

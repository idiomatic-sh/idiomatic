//! The cases of the `definitions` part: a definition is made once for its node and its programs do not depend on which rule
//! asked first; a pattern is asked once; a constant text has one number.

use super::service::{asked, defined, text, use_branch};
use super::types::{Drafts, Flattened};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::described::described;
use crate::domain::lowering::fixtures::{digests_of, named};
use crate::domain::lowering::nodes::types::{AcceptedPlace, NodeHome, StepShape};
use crate::domain::lowering::service::lowered;
use crate::domain::lowering::types::{SelectedRule, SelectedRules};
use crate::domain::steps::types::{ColumnMask, MadeColumn};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::descriptions::types::{FixedName, KindVocabulary, LanguageName};
use ryo_values::domain::plan_parts::types::{LiteralText, StepId};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::query_plans::types::NodeDigests;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// What a row asks of the definitions.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The scenes lowered together in this order, then in the reverse order: whether the two packs' definitions and
    /// tables are said the same, and how many lines say them.
    Orders(&'static [&'static str]),
    /// One node defined twice, two patterns asked of it (one of them twice), two texts written (one of them twice): the
    /// definitions, the patterns of the first, the patterns waiting, the texts.
    Counts,
}

type Input = Asked;
/// What was counted, said.
type Value = Cow<'static, str>;
type Failure = ();

/// The lines of a pack's patterns and pipelines, sorted: the rules' own lines left out.
fn lines(scenes: &[&str]) -> Vec<String> {
    let plans: Vec<_> = scenes.iter().map(|scene| named(scene)).collect();
    let digests: Vec<NodeDigests> = plans
        .iter()
        .map(|plan| digests_of(plan, RuleBindings(&[])))
        .collect();
    let rules: Vec<SelectedRule<'_>> = plans
        .iter()
        .zip(&digests)
        .zip(0..)
        .map(|((plan, digests), rule)| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan,
            bindings: RuleBindings(&[]),
            digests,
        })
        .collect();
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let lowered = lowered(SelectedRules(&rules), None, &kinds);
    let mut lines: Vec<String> = described(&lowered.pack)
        .into_iter()
        .filter(|line| line.starts_with("  pattern") || line.starts_with("  pipeline"))
        .collect();
    lines.sort();
    lines
}

fn counted() -> String {
    let mut drafts = Drafts::default();
    let home = NodeHome {
        rule: AcceptedPlace(0),
        step: StepId(0),
    };
    let shape = || StepShape(vec![CellKind::Node, CellKind::File]);
    let node = NodeDigest([7; 16]);
    let first = defined(&mut drafts, node, home, shape(), Flattened::default());
    let again = defined(&mut drafts, node, home, shape(), Flattened::default());
    let key = ColumnMask([MadeColumn(0)].into());
    let nothing = ColumnMask::default();
    asked(&mut drafts, first, &key, &nothing);
    asked(&mut drafts, first, &nothing, &nothing);
    asked(&mut drafts, first, &key, &nothing);
    // A vocabulary of one fixed name: the empty text is word 0, the fixed name word 1, the pack's own texts after them.
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: vec![FixedName("fixed".into())],
    };
    let written = ["a", "a", "b", "", "fixed"].map(|text| LiteralText(text.into()));
    let numbers = written
        .each_ref()
        .map(|each| text(&mut drafts, &kinds, each));
    let patterns = drafts
        .definitions
        .first()
        .map_or(0, |draft| draft.patterns.len());
    format!(
        "{} definitions (the second {}); {patterns} patterns, {} waiting; words {:?} of {} texts; a use of {} columns",
        drafts.definitions.len(),
        if first == again {
            "is the first"
        } else {
            "is another"
        },
        drafts.waiting.len(),
        numbers.map(|number| number.0),
        drafts.texts.len(),
        use_branch(&mut drafts, first).row.len(),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_definitions_program_is_not_its_callers_two_rule_orders_give_the_same_programs",
        ),
        word: Word::Succeeds,
        input: || Asked::Orders(&["one table, two readers", "pairs within one group"]),
        expect: Expect::Value(Cow::Borrowed("the same in either order: 5 lines")),
    },
    Case {
        name: CaseName(
            "a_node_has_one_definition_a_pattern_is_asked_once_and_a_text_is_its_base_word",
        ),
        word: Word::Succeeds,
        input: || Asked::Counts,
        expect: Expect::Value(Cow::Borrowed(
            "1 definitions (the second is the first); 2 patterns, 2 waiting; words [2, 2, 3, 0, 1] of 2 texts; a use of 2 columns",
        )),
    },
];

fn subject(asked: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match asked {
        Asked::Orders(scenes) => {
            let mut reversed = scenes.to_vec();
            reversed.reverse();
            let (first, second) = (lines(scenes), lines(&reversed));
            let same = if first == second {
                "the same"
            } else {
                "not the same"
            };
            format!("{same} in either order: {} lines", first.len())
        }
        Asked::Counts => counted(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

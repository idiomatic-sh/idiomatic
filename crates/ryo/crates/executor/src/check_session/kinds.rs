//! The words a query writes for a kind, read from its lowered plan, and whether a linked language has a kind each names: the refusal of
//! an unknown kind, made before any file is read.

use super::refusal::Refusal;
use ryo_execution_domain::domain::kind_check::service::{languages_of, names_kind};
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, KindWord, LanguageName};
use ryo_values::domain::finish_plans::expressions::types::{Comparator, PlanConstant, PlanExpression, SourceAlias};
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep, SelectStep};
use ryo_values::domain::plan_parts::types::Test;

/// The relations whose kind column holds the kinds of a language's nodes and tokens.
const KINDED: [&str; 2] = ["syntax-tree", "semantic-syntax"];
const KIND: &str = "kind";

/// The words of `plan` that are compared with the kind of a node or a token, and how.
#[must_use]
pub fn asked(plan: &FinishPlan) -> Vec<(Test, String)> {
    let mut found = Vec::new();
    for step in &plan.steps {
        let FinishStep::Select(select) = step else { continue };
        let kinded: Vec<&SourceAlias> = select
            .sources
            .iter()
            .filter(|source| matches!(plan.steps.get(source.step.0), Some(FinishStep::Base(name)) if KINDED.contains(&&*name.0)))
            .map(|source| &source.alias)
            .collect();
        if kinded.is_empty() {
            continue;
        }
        if let ryo_values::domain::finish_plans::types::RowCondition::Tested(condition) = &select.condition {
            walk(condition, select, &kinded, &mut found);
        }
    }
    found
}

/// Whether `expression` reads the kind column of one of the sources that have one.
fn is_kind(expression: &PlanExpression, select: &SelectStep, kinded: &[&SourceAlias]) -> bool {
    let PlanExpression::Column(read) = expression else { return false };
    &*read.column.0 == KIND
        && match &read.alias {
            alias @ SourceAlias::Written(_) => kinded.contains(&alias),
            SourceAlias::Unwritten => select.sources.len() == 1,
        }
}

/// The texts a constant or a list of them holds.
fn texts(expression: &PlanExpression, out: &mut Vec<String>) {
    match expression {
        PlanExpression::Literal(constant) => constant_texts(constant, out),
        PlanExpression::List(items) => items.iter().for_each(|item| texts(item, out)),
        _ => {}
    }
}

fn constant_texts(constant: &PlanConstant, out: &mut Vec<String>) {
    match constant {
        PlanConstant::Text(text) => out.push(text.0.to_string()),
        PlanConstant::List(items) => items.iter().for_each(|item| constant_texts(item, out)),
        _ => {}
    }
}

fn walk(expression: &PlanExpression, select: &SelectStep, kinded: &[&SourceAlias], found: &mut Vec<(Test, String)>) {
    let mut ask = |test: Test, value: &PlanExpression, found: &mut Vec<(Test, String)>| {
        let mut words = Vec::new();
        texts(value, &mut words);
        found.extend(words.into_iter().map(|word| (test, word)));
    };
    match expression {
        PlanExpression::Compare(comparison) => {
            let test = match comparison.comparator {
                Comparator::Equal => Test::Equal,
                Comparator::NotEqual => Test::NotEqual,
                _ => return,
            };
            if is_kind(&comparison.left, select, kinded) {
                ask(test, &comparison.right, found);
            } else if is_kind(&comparison.right, select, kinded) {
                ask(test, &comparison.left, found);
            }
        }
        PlanExpression::In(pair) | PlanExpression::NotIn(pair) => {
            if is_kind(&pair.left, select, kinded) {
                ask(Test::In, &pair.right, found);
            }
        }
        PlanExpression::Call(call) => {
            let test = match &*call.function.0 {
                "glob_any" => Test::GlobAny,
                "starts_with" => Test::StartsWith,
                "ends_with" => Test::EndsWith,
                "matches" => Test::Matches,
                _ => return,
            };
            if let [value, pattern] = call.arguments.as_slice()
                && is_kind(value, select, kinded)
            {
                ask(test, pattern, found);
            }
        }
        PlanExpression::And(pair) | PlanExpression::Or(pair) => {
            walk(&pair.left, select, kinded, found);
            walk(&pair.right, select, kinded, found);
        }
        PlanExpression::Not(inner) => walk(inner, select, kinded, found),
        PlanExpression::Column(_) | PlanExpression::Literal(_) | PlanExpression::Number(_) | PlanExpression::List(_) => {}
    }
}

/// The refusal of the first word `plan` writes for a kind that none of `languages` (each with the names of its kinds) has, or nothing.
#[must_use]
pub fn unknown(plan: &FinishPlan, languages: impl FnOnce() -> Vec<(String, Vec<String>)>) -> Option<Refusal> {
    let asked = asked(plan);
    if asked.is_empty() {
        return None;
    }
    let languages = languages();
    let vocabularies = KindVocabularies(
        languages
            .iter()
            .map(|(name, kinds)| KindVocabulary {
                language: LanguageName(name.as_str().into()),
                kinds: kinds.iter().map(|kind| KindWord(kind.as_str().into())).collect(),
                fields: Vec::new(),
            })
            .collect(),
    );
    asked
        .into_iter()
        .find(|(test, word)| !names_kind(*test, word, &vocabularies))
        .map(|(_, word)| Refusal::UnknownKind { word, languages: languages_of(&vocabularies).0.to_string() })
}

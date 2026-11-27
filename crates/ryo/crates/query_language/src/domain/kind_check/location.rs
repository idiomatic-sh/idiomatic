//! Where a query writes a word for a kind, for the refusal a person reads: the comparison of the kind column with the word, in the root
//! or in the module that wrote it. The plan that runs has no places, so this reads the program; a word written in more than one
//! comparison has no one place.

use ryo_values::domain::program::service::identity_of;
use ryo_values::domain::program::types::WrittenProgram;
use ryo_values::domain::query_syntax::types::{Condition, Expression, Filter, WrittenOperand};
use ryo_values::domain::query_tokens::types::{QueryOrigin, QuerySpan};

/// Where the comparison is and where its module was written: the origin of the step, with the comparison's span; none for a step of the
/// root, which the refusal then places at the whole query.
pub type Placed = Option<QueryOrigin>;

/// Whether `filter` compares a column named `kind` with the text `word`.
fn compares(filter: &Filter, word: &str) -> bool {
    let kind = |operand: &WrittenOperand| matches!(operand, WrittenOperand::Column(read) if &*read.column.0 == "kind");
    let text = |operand: &WrittenOperand| matches!(operand, WrittenOperand::Text(literal) if &*literal.0 == word);
    (kind(&filter.left) && text(&filter.right)) || (kind(&filter.right) && text(&filter.left))
}

fn collect(conditions: &[Condition], word: &str, found: &mut Vec<(QuerySpan, usize)>, at: usize) {
    for condition in conditions {
        match condition {
            Condition::Test(filter) if compares(filter, word) => found.push((filter.span, at)),
            Condition::Test(_) => {}
            Condition::Any(alternatives) => alternatives.iter().for_each(|each| collect(each, word, found, at)),
            Condition::Not(inner) => collect(inner, word, found, at),
        }
    }
}

/// The one comparison of the kind column with `word`, as the refusal places it, or nothing when there is none or more than one.
#[must_use]
pub fn located(program: &WrittenProgram, word: &str) -> Option<(QuerySpan, Placed)> {
    let mut found = Vec::new();
    for (at, step) in program.steps.iter().enumerate() {
        if let Expression::Select(select) = &step.expression {
            collect(&select.filters, word, &mut found, at);
        }
    }
    let [(span, at)] = found.as_slice() else { return None };
    let origin = identity_of(program.steps[*at].name.1).and_then(|identity| program.step_origins.get(&identity)).cloned();
    Some((*span, origin.map(|mut origin| {
        origin.span = *span;
        origin
    })))
}

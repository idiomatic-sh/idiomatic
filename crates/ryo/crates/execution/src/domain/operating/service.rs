//! The `operating` service: free functions named by verb over what an operator reads, with no IO and no ports.

use crate::domain::operating::types::Family;
use ryo_values::domain::plan_parts::types::Test;

/// The family of comparison a test belongs to.
#[must_use]
pub(crate) fn family_of(test: Test) -> Family {
    match test {
        Test::Equal | Test::NotEqual => Family::Equality,
        Test::NaturalLessThan
        | Test::LessThan
        | Test::LessThanOrEqual
        | Test::GreaterThan
        | Test::GreaterThanOrEqual => Family::Ordering,
        Test::FloatTextGreaterThanOrEqual => Family::FloatText,
        Test::NumericGreaterThanOrEqual | Test::IntegerSuccessor => Family::Numeric,
        Test::In | Test::NotIn => Family::Membership,
        Test::GlobAny | Test::NotGlobAny => Family::Glob,
        Test::StartsWith | Test::EndsWith => Family::Affix,
        Test::Matches => Family::Regex,
    }
}

/// A name the steps made, as a value: when the steps run after the files were read (the text of the workers is
/// given) it is a name that has left its file, which compares with the names the files handed over; before, a name of the file.
#[must_use]
pub(crate) fn minted(
    symbols: &ryo_language::domain::symbols::types::Symbols,
    symbol: ryo_relations::domain::terms::types::Symbol,
    after_the_files: super::types::AfterTheFiles,
) -> ryo_relations::domain::relations::types::Value {
    if after_the_files == super::types::AfterTheFiles::After
        && let Some(name) =
            ryo_language::domain::symbols::service::name_made_across(symbols, symbol)
    {
        return ryo_relations::domain::relations::types::Value::Name(name);
    }
    ryo_relations::domain::relations::types::Value::Symbol(symbol)
}

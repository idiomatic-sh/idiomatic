//! The `syntax` port's contract holds an adapter to account: one that makes no elements fails it.

use super::check;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::TableBuilder;

/// An adapter that makes nothing of any source.
struct Nothing;

impl SyntaxPort for Nothing {
    fn read(&self, _text: SourceText<'_>, _builder: &mut TableBuilder<'_>) {}
}

#[test]
#[should_panic(expected = "a_function_gives_its_elements_in_order")]
fn an_adapter_that_makes_nothing_breaks_the_contract() {
    check(|_| Nothing);
}

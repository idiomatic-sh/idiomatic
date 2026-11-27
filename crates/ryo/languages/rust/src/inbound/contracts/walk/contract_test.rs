//! The `walk` contract's cases: the walk's kind count is the language's, and a token spells its text where an error element does not.

use ryo::languages::{self as l, Count, Walk};

#[test]
fn the_kind_count_is_the_count_of_the_language_s_kinds() {
    fn count<W: Walk>(_: &W) -> Count {
        W::KIND_COUNT
    }
    assert_eq!(count(&crate::walk_current()).0, crate::domain::syntax::service::kinds().count());
}

#[test]
fn a_token_spells_its_text_and_an_error_element_does_not() {
    use crate::domain::syntax::{service::kind_bits, types::Kind};
    use crate::domain::tokens::types::Token;
    fn spells<W: Walk + ryo::languages::Fillers>(_: &W, kind: l::KindBits) -> bool {
        <W as ryo::languages::Fillers>::spells(kind)
    }
    let walk = crate::walk_current();
    assert!(spells(&walk, kind_bits(Kind::Token(Token::Ident))));
    assert!(!spells(&walk, kind_bits(Kind::Token(Token::Error))));
}

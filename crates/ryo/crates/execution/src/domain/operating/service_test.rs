//! Comparison-family classification and project text identity are pure operating contracts.
use super::{
    service::{family_of, minted},
    types::{AfterTheFiles, Family},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::plan_parts::types::Test,
};
#[derive(Clone, Copy)]
enum Scenario {
    Family,
    ProjectName,
}
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_test_belongs_to_its_family"),
        word: Word::Succeeds,
        input: || Scenario::Family,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_name_made_after_the_files_were_read_compares_with_theirs"),
        word: Word::Succeeds,
        input: || Scenario::ProjectName,
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::Family => assert_eq!(family_of(Test::NotGlobAny), Family::Glob),
        Scenario::ProjectName => {
            use ryo_language::domain::symbols::{
                service::{hash_in, intern_derived, place_of},
                types::{ByteSlice, Symbols},
            };
            use ryo_relations::domain::{
                relations::types::{CROSS_FILE, Origin, Value},
                terms::types::NameText,
            };
            let mut symbols = Symbols::default();
            let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("here"));
            assert_eq!(
                minted(&symbols, symbol, AfterTheFiles::Before),
                Value::Symbol(symbol)
            );
            let Value::Name(name) = minted(&symbols, symbol, AfterTheFiles::After) else {
                panic!("project text is a name")
            };
            assert_eq!(Some(name.hash), hash_in(&symbols, symbol));
            assert_eq!(
                place_of(name, &[], &symbols).unwrap().origin,
                Origin::Derived(CROSS_FILE)
            );
            let texts = crate::domain::table_texts::types::Texts {
                symbols: &symbols,
                source: ryo_values::domain::files::types::SourceView(b""),
                paths: &[],
                arenas: &[],
            };
            assert_eq!(
                crate::domain::table_texts::service::datum(Value::Name(name), &texts),
                crate::domain::table_texts::types::Datum::Text(
                    crate::domain::table_texts::types::TextView("here")
                )
            );
        }
    }
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

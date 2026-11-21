//! Where a compile sites the statements above linear, from the compile inbound API over the real parser and real modules: a
//! site is the statement's document and the name its `let` gives there. It is the same in every query that imports the
//! document, whatever the query passes, it is one finding however often the query expands it, and it does not move when a line
//! above it or a query's own statements change. A statement of the query's own document is sited there, and a statement no
//! root reads is not classified.

use super::super::{Document, costed, rendered};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// A module with an input and a parameter that pairs its input with the comments its parameter keeps, on the file alone, and
/// returns the pairs.
const PAIRING: &str = "input candidates: relation<id: node, file: file>;\nparam codes: string;\noutput relation<id: node, file: file, other: node>;\nlet coded = select id, file from \"comment\" where matches(text, $codes);\nlet pairs = select l.id as id, l.file as file, r.id as other from candidates as l join coded as r on l.file = r.file;\nreturn pairs;";
/// The same module with a statement written above its pairs.
const PAIRING_EDITED_ABOVE: &str = "input candidates: relation<id: node, file: file>;\nparam codes: string;\noutput relation<id: node, file: file, other: node>;\nlet coded = select id, file from \"comment\" where matches(text, $codes);\nlet spare = select id, file from \"comment\";\nlet pairs = select l.id as id, l.file as file, r.id as other from candidates as l join coded as r on l.file = r.file;\nreturn pairs;";
/// A module that calls the pairing module and returns what it returns.
const WRAPPING: &str = "import pairing from \"pack/pairing\";\ninput candidates: relation<id: node, file: file>;\noutput relation<id: node, file: file, other: node>;\nlet wrapped = pairing(candidates=candidates, codes=\"^W\");\nreturn wrapped;";

/// A rule that passes the functions and one pattern.
const OVER_FUNCTIONS: &str = "import pairing from \"pack/pairing\";\nlet functions = select id, file from \"function\";\nlet found = pairing(candidates=functions, codes=\"^F\");\nemit found subject file key(id, other);";
/// A rule that passes the comments of some text, a different sub-plan, and another pattern.
const OVER_COMMENTS: &str = "import pairing from \"pack/pairing\";\nlet noted = select id, file from \"comment\" where text != \"\";\nlet narrowed = select id, file from noted where id = id;\nlet found = pairing(candidates=narrowed, codes=\"^C\");\nemit found subject file key(id, other);";
/// A rule that calls the module twice, with two patterns.
const TWICE: &str = "import pairing from \"pack/pairing\";\nlet functions = select id, file from \"function\";\nlet first = pairing(candidates=functions, codes=\"^A\");\nlet second = pairing(candidates=functions, codes=\"^B\");\nlet found = union(left=first, right=second);\nemit found subject file key(id, other);";
/// A rule that reaches the pairs through a module that calls the pairing module.
const THROUGH_A_WRAPPER: &str = "import wrapping from \"pack/wrapping\";\nlet functions = select id, file from \"function\";\nlet found = wrapping(candidates=functions);\nemit found subject file key(id, other);";
/// A rule with a product of its own, named `pairs`.
const OWN_PRODUCT: &str = "let pairs = select l.id as id, l.file as file, r.id as other from \"function\" as l join \"comment\" as r on l.file = r.file;\nemit pairs subject file key(id, other);";
/// A rule whose product no root reads.
const UNREAD_PRODUCT: &str = "let pairs = select l.id as id, l.file as file, r.id as other from \"function\" as l join \"comment\" as r on l.file = r.file;\nlet functions = select id, file from \"function\";\nemit functions subject file key(id);";

/// What a row asks: the cost of each of these rules over these modules, one after the other.
#[derive(Clone, Copy)]
struct Ask(&'static [Document], &'static [Document]);

type Input = Ask;
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

/// The pairing module under its name.
const MODULES: &[Document] = &[Document("pack/pairing", PAIRING)];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_rules_that_pass_different_inputs_and_values_reach_one_site"),
        word: Word::Succeeds,
        input: || {
            Ask(
                &[
                    Document("rules/A", OVER_FUNCTIONS),
                    Document("rules/B", OVER_COMMENTS),
                ],
                MODULES,
            )
        },
        expect: Expect::Value(Cow::Borrowed(
            "rules/A: QuadraticInFile [pack/pairing::pairs Module Product QuadraticInFile] | rules/B: QuadraticInFile [pack/pairing::pairs Module Product QuadraticInFile]",
        )),
    },
    Case {
        name: CaseName("a_module_expanded_twice_in_one_rule_is_one_finding"),
        word: Word::Succeeds,
        input: || Ask(&[Document("rules/A", TWICE)], MODULES),
        expect: Expect::Value(Cow::Borrowed(
            "rules/A: QuadraticInFile [pack/pairing::pairs Module Product QuadraticInFile]",
        )),
    },
    Case {
        name: CaseName("a_statement_written_above_does_not_move_a_site"),
        word: Word::Succeeds,
        input: || {
            Ask(
                &[Document("rules/A", OVER_FUNCTIONS)],
                &[Document("pack/pairing", PAIRING_EDITED_ABOVE)],
            )
        },
        expect: Expect::Value(Cow::Borrowed(
            "rules/A: QuadraticInFile [pack/pairing::pairs Module Product QuadraticInFile]",
        )),
    },
    Case {
        name: CaseName("the_statement_a_module_returns_keeps_the_name_the_module_gives_it"),
        word: Word::Succeeds,
        input: || {
            Ask(
                &[Document("rules/A", THROUGH_A_WRAPPER)],
                &[
                    Document("pack/pairing", PAIRING),
                    Document("pack/wrapping", WRAPPING),
                ],
            )
        },
        expect: Expect::Value(Cow::Borrowed(
            "rules/A: QuadraticInFile [pack/pairing::pairs Module Product QuadraticInFile]",
        )),
    },
    Case {
        name: CaseName("a_rules_own_product_is_sited_in_its_own_document"),
        word: Word::Succeeds,
        input: || Ask(&[Document("rules/A", OWN_PRODUCT)], &[]),
        expect: Expect::Value(Cow::Borrowed(
            "rules/A: QuadraticInFile [rules/A::pairs Query Product QuadraticInFile]",
        )),
    },
    Case {
        name: CaseName("a_statement_no_root_reads_is_not_classified"),
        word: Word::Succeeds,
        input: || Ask(&[Document("rules/A", UNREAD_PRODUCT)], &[]),
        expect: Expect::Value(Cow::Borrowed("rules/A: Linear []")),
    },
];

fn subject(ask: Input) -> Result<Value, Failure> {
    let Ask(rules, modules) = ask;
    let costs = rules
        .iter()
        .map(|rule| format!("{}: {}", rule.0, rendered(&costed(*rule, modules))))
        .collect::<Vec<_>>();
    Ok(Cow::Owned(costs.join(" | ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

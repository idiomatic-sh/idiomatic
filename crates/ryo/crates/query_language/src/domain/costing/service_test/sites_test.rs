//! `sited` over modules written by hand: a statement of a module is sited at the name the module's own `let` gives it, whatever
//! name its expansion carries and whatever path of imports reached it; a statement of a document that is no module is sited at
//! its own name, in the query's own document; and each statement of a module is found by one search.

use super::super::service::sited;
use super::super::types::SiteHome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::program::types::{
    FragmentProgram, FragmentReturn, LetStep, ParsedModules, RelationSignature,
};
use ryo_values::domain::query_syntax::types::{Call, Expression};
use ryo_values::domain::query_tokens::types::{
    ByteOffset, OperationName, QueryOrigin, QuerySourceId, QuerySpan, QueryTraceSite,
    StepIdentified, StepName, StepSpelling,
};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Ask {
    /// The site of the second statement of a module of three, expanded under its caller's name and reached through an import.
    ReturnedUnderTheCallersName,
    /// The site of a statement of a document that is no module.
    OfTheQuery,
    /// The statements of a module of so many statements and of one of twice as many, each asked once.
    Counted(usize),
}

type Input = Ask;
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_modules_statement_keeps_the_name_the_module_gives_it"),
        word: Word::Succeeds,
        input: || Ask::ReturnedUnderTheCallersName,
        expect: Expect::Value(Cow::Borrowed("pack/module::step_1 Module")),
    },
    Case {
        name: CaseName("a_statement_of_the_query_is_sited_in_its_own_document"),
        word: Word::Succeeds,
        input: || Ask::OfTheQuery,
        expect: Expect::Value(Cow::Borrowed("rules/A::the_callers_name Query")),
    },
    Case {
        name: CaseName("each_statement_of_a_module_is_sited_by_one_search"),
        word: Word::Succeeds,
        input: || Ask::Counted(16),
        expect: Expect::Value(Cow::Borrowed(
            "16 statements: 16 sited at their own name; 32 statements: 32 sited at their own name",
        )),
    },
];

/// The span of the statement at `place` of a document: ten bytes each.
fn span(place: usize) -> QuerySpan {
    QuerySpan {
        start: ByteOffset(place.saturating_mul(10)),
        end: ByteOffset(place.saturating_mul(10).saturating_add(9)),
    }
}

/// A statement named `name`, written at `place`.
fn statement(name: &str, place: usize) -> LetStep {
    LetStep {
        name: StepName(StepSpelling(name.into()), StepIdentified::Unassigned),
        expression: Expression::Call(Call {
            name: OperationName("union".into()),
            arguments: Vec::new(),
            span: span(place),
        }),
        span: span(place),
    }
}

/// The modules of a catalog that holds one, `pack/module`, of `count` statements named `step_0`, `step_1` and so on.
fn modules(count: usize) -> ParsedModules {
    let signature = || RelationSignature {
        columns: Vec::new(),
        span: span(0),
    };
    let module = FragmentProgram {
        imports: Vec::new(),
        inputs: Vec::new(),
        parameters: Vec::new(),
        output: signature(),
        output_span: span(0),
        steps: (0..count)
            .map(|place| statement(&format!("step_{place}"), place))
            .collect(),
        result: FragmentReturn {
            step: StepName(StepSpelling("step_0".into()), StepIdentified::Unassigned),
            span: span(count),
        },
    };
    ParsedModules(Arc::new(BTreeMap::from([(
        QuerySourceId("pack/module".into()),
        module,
    )])))
}

/// Where the statement written at `place` of `document` came from, reached through one import of a rule.
fn origin(document: &str, place: usize) -> QueryOrigin {
    QueryOrigin {
        source_id: QuerySourceId(document.into()),
        span: span(place),
        trace: Arc::new([QueryTraceSite {
            source_id: QuerySourceId("rules/A".into()),
            target_id: QuerySourceId(document.into()),
            span: span(7),
        }]),
    }
}

/// How many statements of a module of `count` statements are sited at the name the module gives them, each asked once under
/// a name that is not the module's.
fn counted(count: usize) -> String {
    let modules = modules(count);
    let expanded = statement("the_callers_name", 0);
    let placed = (0..count)
        .filter(|place| {
            let site = sited(&origin("pack/module", *place), &expanded, &modules);
            *site.site.statement == format!("step_{place}") && site.home == SiteHome::Module
        })
        .count();
    format!("{count} statements: {placed} sited at their own name")
}

fn subject(ask: Input) -> Result<Value, Failure> {
    let expanded = statement("the_callers_name", 0);
    let site = match ask {
        Ask::ReturnedUnderTheCallersName => {
            sited(&origin("pack/module", 1), &expanded, &modules(3))
        }
        Ask::OfTheQuery => sited(&origin("rules/A", 1), &expanded, &modules(3)),
        Ask::Counted(count) => {
            let twice = counted(count.saturating_mul(2));
            return Ok(Cow::Owned(format!("{}; {twice}", counted(count))));
        }
    };
    Ok(Cow::Owned(format!(
        "{}::{} {:?}",
        site.site.document.0, site.site.statement, site.home
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

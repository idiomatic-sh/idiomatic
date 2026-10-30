//! What the `contract_rendering` service makes of a rendered content, on values alone.

use super::service::rendered;
use super::types::{ContractRequest, Rendered};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contexts::types::RenderContext;
use crate::domain::contracts::types::TemplateIdentity;
use crate::domain::definitions::types::{Exactness, TemplateText};
use crate::domain::files::types::FileText;
use crate::domain::paths::types::RepoPath;

/// What the template engine answered for the content.
#[derive(Clone, Copy)]
enum Scenario {
    /// A content that renders to text.
    Text,
    /// A content that renders to whitespace.
    Blank,
    /// A content that renders to whitespace, of a verbatim template.
    BlankVerbatim,
    /// A content that does not render.
    Refused,
}

type Input = Scenario;
/// What the rendering made of the file, written out.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rendered_content_is_the_contract_of_its_file"),
        word: Word::Succeeds,
        input: || Scenario::Text,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_blank_content_asks_nothing_of_its_file"),
        word: Word::Succeeds,
        input: || Scenario::Blank,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_blank_verbatim_content_is_the_contract_of_its_file"),
        word: Word::Succeeds,
        input: || Scenario::BlankVerbatim,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_content_that_does_not_render_is_a_template_error_of_its_file"),
        word: Word::Succeeds,
        input: || Scenario::Refused,
        expect: Expect::Value(String::new()),
    },
];

fn request() -> ContractRequest {
    ContractRequest {
        target: RepoPath("src/lib.txt".into()),
        rendered: RepoPath(".tastemaker-rendered/0.lib.txt".into()),
        identity: TemplateIdentity("p/root".into()),
        exact: Exactness::Contains,
        content: TemplateText("x".into()),
        context: RenderContext::default(),
    }
}

/// What the scenario gave, and what it should give.
fn said(scenario: Scenario) -> (String, &'static str) {
    let made = match scenario {
        Scenario::Text => rendered(request(), Ok(FileText("fn f() {}".into()))),
        Scenario::Blank => rendered(request(), Ok(FileText(" \n".into()))),
        Scenario::BlankVerbatim => rendered(
            ContractRequest {
                exact: Exactness::Verbatim,
                ..request()
            },
            Ok(FileText(" \n".into())),
        ),
        Scenario::Refused => rendered(request(), Err(&"no value for `name`")),
    };
    let shown = match made {
        Rendered::Contract(contract) => format!(
            "{} {} {} {}",
            contract.target.0, contract.rendered.0, contract.identity.0, contract.text.0
        ),
        Rendered::Nothing => "nothing".to_owned(),
        Rendered::Failed(finding) => {
            format!(
                "{} {} {}",
                crate::domain::findings::service::printed(&finding.rule).0,
                finding.path.0,
                finding.message.0
            )
        }
    };
    let wanted = match scenario {
        Scenario::Text => "src/lib.txt .tastemaker-rendered/0.lib.txt p/root fn f() {}",
        Scenario::Blank => "nothing",
        Scenario::BlankVerbatim => "src/lib.txt .tastemaker-rendered/0.lib.txt p/root  \n",
        Scenario::Refused => {
            "p/root#template src/lib.txt the `p/root` template does not render here: no value for `name`"
        }
    };
    (shown, wanted)
}

fn subject(scenario: Input) -> Value {
    let (said, wanted) = said(scenario);
    if said == wanted { String::new() } else { said }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

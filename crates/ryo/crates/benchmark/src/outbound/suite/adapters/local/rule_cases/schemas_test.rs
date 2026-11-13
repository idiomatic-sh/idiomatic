//! The accepted and refused cases documents.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static str;
type Value = bool;
type Failure = &'static str;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("full report accepts explicit frontend identity"),
        word: Word::Succeeds,
        input: || "[[cases]]\nname='parser'\ncomparison='full'\nfiles={'case.py'='x'}\nexpected=[{rule='invalid-syntax',file='case.py',start=0,end=1}]",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("full report retains inherited identity for policy admission"),
        word: Word::Succeeds,
        input: || "[[cases]]\nname='parser'\ncomparison='full'\nfiles={'case.py'='x'}\nexpected=[{file='case.py',start=0,end=1}]",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("comparison scope rejects unknown modes"),
        word: Word::Refuses,
        input: || "[[cases]]\nname='parser'\ncomparison='partial'\nfiles={'case.py'='x'}\nexpected=[]",
        expect: Expect::Error("invalid report envelope"),
    },
    Case {
        name: CaseName("a context carries binary auxiliary bytes"),
        word: Word::Succeeds,
        input: || "[[cases]]\nname='context'\nfiles={'mod.py'='import pkg.mod'}\ncontext={logical-root='pkg',auxiliary-files={'pkg/__init__.py'=[0,1,255]},missing=['__init__.py']}\nexpected=[]\n",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a declared project inventory keeps its entry state"),
        word: Word::Succeeds,
        input: || "[[cases]]\nname='sibling'\ncomparison='full'\nrequested-files=['main.py']\nproject-search-roots=['']\nfiles={'main.py'='import localmod', 'localmod.py'='value=1'}\ngiven-project-inventory={completeness='partial', entries=[{path='localmod.py', state='file'}]}\nexpected=[]\n",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an inventory entry cannot leave the project"),
        word: Word::Refuses,
        input: || "[[cases]]\nname='sibling'\ncomparison='full'\nrequested-files=['main.py']\nproject-search-roots=['']\nfiles={'main.py'='import localmod', 'localmod.py'='value=1'}\ngiven-project-inventory={completeness='partial', entries=[{path='../bad.py', state='file'}]}\nexpected=[]\n",
        expect: Expect::Error("invalid report envelope"),
    },
];
/// Decodes one cases document and checks what the row names.
fn subject(text: Input) -> Result<Value, Failure> {
    let cases = super::cases(text).map_err(|_refusal| "invalid report envelope")?;
    let case = cases.cases.first().expect("one decoded case");
    let identity_preserved = match (
        case.expected.first().map(|expected| &expected.identity),
        text.contains("rule="),
    ) {
        (
            Some(ryo_benchmark_contracts::domain::workloads::types::ExpectedRule::Explicit(rule)),
            true,
        ) => rule.as_ref() == "invalid-syntax",
        (Some(ryo_benchmark_contracts::domain::workloads::types::ExpectedRule::Parent), false) => {
            true
        }
        (None, _) => true,
        _ => false,
    };
    let comparison = if text.contains("comparison='full'") {
        ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Full
    } else {
        ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Rule
    };
    let context_bytes = !text.contains("auxiliary-files")
        || matches!(&case.context, ryo_benchmark_contracts::domain::workloads::types::ContextDeclaration::Declared(context)
            if context.auxiliary_files[std::path::Path::new("pkg/__init__.py")].0 == [0, 1, 255]);
    let inventory = !text.contains("given-project-inventory")
        || matches!(&case.given_project_inventory, ryo_benchmark_contracts::domain::workloads::types::InventoryDeclaration::Declared(inventory)
            if inventory.entries.len() == 1
                && inventory.entries[0].state == ryo::linter::ProjectEntryState::File);
    Ok(case.comparison == comparison && identity_preserved && context_bytes && inventory)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

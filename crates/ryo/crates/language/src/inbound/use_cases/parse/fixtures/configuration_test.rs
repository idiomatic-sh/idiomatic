//! Selection precedes every read, and independent inputs retain their own selected parser.
use super::super::{
    expected::{Leafness, Shape, Walked},
    findings::Findings,
    rows::rows_of_inputs_with,
};
use super::{ConfiguredFixture, Selection};
use crate::domain::elements::types::{Completion, TableBuilder, Tree};
use crate::inbound::use_cases::parse::{
    interface::ParseUseCase,
    types::{KindNames, ParseConfigurationIdentity, ParseRequest, Recognition, SkippedDirectory},
};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
use ryo_relations::domain::terms::types::UnknownName;
use std::{cell::RefCell, sync::Arc};

/// Accepts only the source matching its selected immutable setting.
struct SelectedParser(&'static str);
impl ParseUseCase for SelectedParser {
    type FileModel = ();

    fn begin_semantics(
        &self,
        _request: &crate::inbound::use_cases::parse::types::ParseRequest<'_>,
        _tree: &crate::domain::elements::types::Elements,
        _slot: &mut crate::domain::semantics::types::SemanticSlot,
    ) {
    }

    fn completion_requirement(&self) -> crate::domain::elements::types::CompletionRequirement {
        crate::domain::elements::types::CompletionRequirement::Independent
    }

    fn extend_file_model(
        &self,
        _tree: &crate::domain::elements::types::Tree<'_>,
        _from: ryo_relations::domain::relations::types::Position,
        _model: &mut (),
    ) {
    }

    fn recognizes_explicit(
        &self,
        _request: &crate::inbound::use_cases::parse::types::ParseRequest<'_>,
    ) -> crate::inbound::use_cases::parse::types::Recognition {
        crate::inbound::use_cases::parse::types::Recognition::Declines
    }
    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::capabilities::types::OptionalCapability] {
        &[]
    }
    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity(self.0)
    }
    fn file_model(&self, _tree: &Tree<'_>, _model: &mut ()) {}
    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        if request.bytes.0 == self.0.as_bytes() {
            Recognition::Reads
        } else {
            Recognition::Declines
        }
    }
    fn skips(&self) -> &'static [SkippedDirectory] {
        &[]
    }
    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }
    fn kind_count(&self) -> Count {
        Count(0)
    }
    fn kind_names(&self) -> KindNames {
        KindNames(Vec::new())
    }
    fn read_tree(
        &self,
        _request: &ParseRequest<'_>,
        _builder: &mut TableBuilder<'_>,
    ) -> Result<(), crate::inbound::use_cases::parse::errors::ParseError> {
        Ok(())
    }
    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {}
    ryo_relations::each_relation!(absent_syntax);
}

/// References are not invoked by a rows-only read.
fn shape(_text: &str) -> Walked {
    Walked {
        kind: KindBits(0),
        leaf: Leafness::Node,
        start: Offset(0),
        end: Offset(0),
        children: Vec::new(),
    }
}

#[test]
fn singleton_and_multiinput_forward_their_own_target_once() -> std::io::Result<()> {
    let root = std::env::temp_dir().join(format!("ryo-selected-fixtures-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let first = root.join("input.first.t");
    let second = root.join("input.second.t");
    std::fs::write(&first, "first")?;
    std::fs::write(&second, "second")?;
    let calls = RefCell::new(Vec::new());
    let configure = |text: &str| {
        calls.borrow_mut().push(text.to_owned());
        let setting = match text {
            "first" => "first",
            "second" => "second",
            _ => return Err("unsupported setting".to_owned()),
        };
        Ok(Some(ConfiguredFixture {
            parse: Arc::new(SelectedParser(setting)),
            shape: Shape::Walk(shape),
            oracle: |_, _| Vec::new(),
        }))
    };
    let selection = Selection {
        default: &crate::inbound::use_cases::parse::use_case::ParseUseCaseImpl,
        configure: &configure,
    };
    for inputs in [vec![first.clone()], vec![first.clone(), second.clone()]] {
        calls.borrow_mut().clear();
        let mut findings = Findings(Vec::new());
        let rows = rows_of_inputs_with(
            &selection,
            &inputs,
            ryo_relations::domain::relations::types::Relation::Entry,
            "entry",
            &mut findings,
        );
        assert!(rows.is_some(), "{:?}", findings.0);
        assert!(findings.0.is_empty());
        assert_eq!(calls.borrow().len(), inputs.len());
        assert_eq!(calls.borrow().first().map(String::as_str), Some("first"));
        if inputs.len() == 2 {
            assert_eq!(calls.borrow().last().map(String::as_str), Some("second"));
        }
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn invalid_declaration_becomes_a_finding_before_parser_use() {
    let calls = std::cell::Cell::new(0);
    let configure = |_: &str| {
        calls.set(calls.get() + 1);
        Err("unsupported target".to_owned())
    };
    let selection = Selection {
        default: &crate::inbound::use_cases::parse::use_case::ParseUseCaseImpl,
        configure: &configure,
    };
    let mut findings = Findings(Vec::new());
    assert!(selection.select("bad", "input.t", &mut findings).is_err());
    assert_eq!(calls.get(), 1);
    assert_eq!(
        findings.0,
        ["input.t: invalid fixture configuration: unsupported target"]
    );
}

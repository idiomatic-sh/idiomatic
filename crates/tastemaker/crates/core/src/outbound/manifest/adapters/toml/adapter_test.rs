//! The TOML adapter of the manifest port: the contract, then the formats as written.

use super::adapter::ManifestTomlAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::types::Role;
use crate::domain::entries::types::ImportPattern;
use crate::domain::topics::service::{keys, topics};
use crate::domain::topics::types::{Necessity, TopicBody};
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::fixtures::contract::check;
use crate::outbound::manifest::interface::ManifestPort as _;
use crate::outbound::manifest::types::DefinitionText;

/// Which definition is read, and its text.
type Input = (Kind, &'static str);
/// What was read, in a word.
type Value = String;
type Failure = ManifestError;

/// The kind of definition a row reads.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Workspace,
    /// A local `tastemaker.toml` whose `imports-allow` is a list of patterns.
    AllowListed,
    /// One whose `imports-allow` names a variable of the file.
    AllowVariable,
    Spec,
    Rule,
    /// A `version` line in each definition, a `tastemaker.toml`, a `package.toml` and a lock: the text is ignored.
    Versioned,
    Template,
    /// A `tastemaker.toml` whose entry writes its version, which the lock pins.
    Pinned,
    /// Each form topic's skeleton, the text ignored.
    Skeletons,
}

const WORKSPACE: &str = r#"workspace = { name = "w", dependencies = [{ package = "p", path = "../p" }] }
specs = [
  { spec = "p/crate", include = ["crates/w/{crate}", { part = "use-case", recursive = true }] },
]
"#;

const SPEC: &str = r#"description = "d"
why = "w"
templates = [
  { template = "errors", file = "errors.txt", count = { when = { condition = "has" }, then = 1, else = 0 }, description = "e" },
]
conditions = { has = { description = "c", rule = { rule = "present", include = ["errors.txt"] } } }
"#;

const RULE: &str = r#"description = "d"
why = "w"
query = "let x = select * from \"comment\"; emit x subject \"file\" key(\"span\");"

[diagnostic]
primary = "span"
message = "m"
help = "h"

[[scenarios]]
id = "plain-comment"
scenario = "A plain comment is reported."
given.include = ["**"]
given.files."src/lib.txt" = "// x\n"
then.findings = [{ path = "src/lib.txt", line = 1 }]
"#;

const TEMPLATE: &str = r#"description = "d"
why = "w"
match = "declarations"
content = '''
pub enum {{ part | pascal }}Error {}
'''
"#;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_root_tastemaker_toml_with_a_part_selector_is_read"),
        word: Word::Succeeds,
        input: || (Kind::Workspace, WORKSPACE),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_spec_with_an_owned_condition_is_read"),
        word: Word::Succeeds,
        input: || (Kind::Spec, SPEC),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_with_a_scenario_is_read"),
        word: Word::Succeeds,
        input: || (Kind::Rule, RULE),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_has_its_content_inline"),
        word: Word::Succeeds,
        input: || (Kind::Template, TEMPLATE),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_imports_allow_of_patterns_is_read"),
        word: Word::Succeeds,
        input: || (Kind::AllowListed, "imports-allow = [\"winnow::*\"]\n"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_imports_allow_naming_a_variable_is_read"),
        word: Word::Succeeds,
        input: || {
            (
                Kind::AllowVariable,
                "variables = { parsers = [\"winnow::*\"] }\nimports-allow = [\"std::*\", { variable = \"parsers\" }]\n",
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("every_form_topic_skeleton_is_read_with_its_required_keys"),
        word: Word::Succeeds,
        input: || (Kind::Skeletons, ""),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_entry_without_include_is_refused_naming_the_fix"),
        word: Word::Refuses,
        input: || (Kind::Pinned, "rules = [{ rule = \"p/r\" }]\n"),
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Shape { reason } if reason.contains("names no `include`"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_spec_entry_without_include_is_refused_naming_the_fix"),
        word: Word::Refuses,
        input: || (Kind::Pinned, "specs = [{ spec = \"p/s\" }]\n"),
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Shape { reason } if reason.contains("names no `include`"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_unknown_key_is_refused"),
        word: Word::Refuses,
        input: || (Kind::Spec, "description = \"d\"\nwhy = \"w\"\nlayer = []\n"),
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Shape {reason} if reason.contains("unknown field `layer`"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_imports_default_other_than_deny_is_refused_not_read_as_allow"),
        word: Word::Refuses,
        input: || {
            (
                Kind::Spec,
                "description = \"d\"\nwhy = \"w\"\nspecs = [\n  { spec = \"part\", include = [\"src\"], imports = { default = \"denied\" }, description = \"d\" },\n]\n",
            )
        },
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Shape { reason } if reason.contains("`imports.default = \"denied\"`"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_unknown_key_lists_the_keys_and_the_topic"),
        word: Word::Refuses,
        input: || {
            (
                Kind::Rule,
                "description = \"d\"\nwhy = \"w\"\nseverity = \"error\"\n",
            )
        },
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Shape { reason } if reason.contains("unknown field `severity`")
                && reason.contains("`query`")
                && !reason.contains("`category`")
                && reason.contains("`tm explain rule`"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_file_that_writes_a_format_version_is_refused_whatever_its_kind"),
        word: Word::Succeeds,
        input: || (Kind::Versioned, ""),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_entry_of_a_tastemaker_toml_writing_its_version_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Kind::Pinned,
                "rules = [{ rule = \"p/r@0.1.0\", include = [\"**\"] }]\n",
            )
        },
        expect: Expect::Refusal(|actual| {
            if matches!(actual, ManifestError::Shape { reason } if reason.contains("tastemaker.lock pins it"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_level_in_a_rule_is_refused_since_every_finding_is_an_error"),
        word: Word::Refuses,
        input: || {
            (
                Kind::Rule,
                "description = \"d\"\nwhy = \"w\"\nlevel = \"warning\"\n",
            )
        },
        expect: Expect::Refusal(level_refused),
    },
    Case {
        name: CaseName("a_level_in_a_rule_scenario_is_refused_since_every_finding_is_an_error"),
        word: Word::Refuses,
        input: || {
            (
                Kind::Rule,
                "description = \"d\"\nwhy = \"w\"\nquery = \"q\"\n\n[diagnostic]\nprimary = \"span\"\nmessage = \"m\"\nhelp = \"h\"\n\n[[scenarios]]\nid = \"s\"\nscenario = \"S.\"\ngiven.level = \"note\"\n",
            )
        },
        expect: Expect::Refusal(level_refused),
    },
    Case {
        name: CaseName("a_level_in_a_spec_entry_is_refused_since_every_finding_is_an_error"),
        word: Word::Refuses,
        input: || {
            (
                Kind::Spec,
                "description = \"d\"\nwhy = \"w\"\nrules = [{ rule = \"r\", include = [\"**\"], level = \"off\" }]\n",
            )
        },
        expect: Expect::Refusal(level_refused),
    },
    Case {
        name: CaseName(
            "a_level_in_a_tastemaker_toml_entry_is_refused_since_every_finding_is_an_error",
        ),
        word: Word::Refuses,
        input: || {
            (
                Kind::Pinned,
                "rules = [{ rule = \"p/r\", include = [\"**\"], level = \"warning\" }]\n",
            )
        },
        expect: Expect::Refusal(level_refused),
    },
];

/// Whether `error` refuses a `level` key, saying every finding is an error.
fn level_refused(error: &ManifestError) -> crate::cases::Selection {
    if matches!(error, ManifestError::Shape { reason } if reason.contains("unknown field `level`")
        && reason.contains("every finding is an error"))
    {
        crate::cases::Selection::Matches
    } else {
        crate::cases::Selection::Differs
    }
}

/// Whether every reader refuses a text that writes a format version, saying to remove the line: no file has one.
fn versions_are_refused(adapter: &ManifestTomlAdapter) -> bool {
    let text = DefinitionText(concat!("version", " = 3\n").into());
    let refused = |answer: Option<ManifestError>| matches!(answer, Some(ManifestError::Shape { reason }) if reason.contains("`version` is no key"));
    refused(adapter.rule(&text).err())
        && refused(adapter.spec(&text).err())
        && refused(adapter.template(&text).err())
        && refused(adapter.workspace_file(&text).err())
        && refused(adapter.package(&text).err())
}

fn subject((kind, text): Input) -> Result<Value, Failure> {
    let text = DefinitionText(text.into());
    let adapter = ManifestTomlAdapter;
    let fine = match kind {
        Kind::Workspace => {
            let file = adapter.workspace_file(&text)?;
            matches!(file.role, Role::Root(_))
                && file.composition.specs[0].header.include.0.len() == 2
        }
        Kind::Spec => {
            let spec = adapter.spec(&text)?;
            spec.composition.conditions.len() == 1 && spec.composition.templates.len() == 1
        }
        Kind::AllowListed => {
            adapter.workspace_file(&text)?.composition.imports_allow
                == [ImportPattern("winnow::*".into())]
        }
        Kind::AllowVariable => {
            let composition = adapter.workspace_file(&text)?.composition;
            composition.imports_allow == [ImportPattern("std::*".into())]
                && composition
                    .imports_allow_variables
                    .iter()
                    .map(|name| &*name.0)
                    .eq(["parsers"])
        }
        Kind::Rule => adapter.rule(&text)?.scenarios.len() == 1,
        Kind::Versioned => versions_are_refused(&adapter),
        Kind::Skeletons => skeletons_are_read(),
        Kind::Template => !adapter.template(&text)?.content.0.is_empty(),
        Kind::Pinned => !adapter.workspace_file(&text)?.composition.rules.is_empty(),
    };
    Ok(if fine {
        String::new()
    } else {
        format!("{kind:?} misread")
    })
}

#[test]
fn succeeds() {
    check(&ManifestTomlAdapter);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// Whether each form topic's skeleton is a file the reader accepts, holding every key its file's table requires.
fn skeletons_are_read() -> bool {
    let refused = topics()
        .iter()
        .filter_map(|topic| match topic.body {
            TopicBody::Form(form) => Some((topic.name.0, form)),
            TopicBody::Text(_) => None,
        })
        .filter(|(name, form)| {
            let text = DefinitionText(form.skeleton.0.into());
            let adapter = ManifestTomlAdapter;
            let refusal = match *name {
                "rule" => adapter.rule(&text).err(),
                "spec" => adapter.spec(&text).err(),
                _ => adapter.template(&text).err(),
            };
            let required = form.tables.first().map(keys).unwrap_or_default();
            refusal.is_some()
                || required.iter().any(|key| {
                    key.presence == Necessity::Required
                        && !form.skeleton.0.lines().any(|line| {
                            line.starts_with(key.name.0)
                                || line.starts_with(&format!("[{}]", key.name.0))
                        })
                })
        })
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    refused.is_empty()
}

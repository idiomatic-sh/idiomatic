// ignore: idiomatic.arch/file-length - the schemas are one wire format read in both directions, and their test holds the rows of both beside the probe deserializer that reads which fields a struct declares.
//! What the schemas accept and refuse: selectors, counts and param values as written.

use super::super::constants::SPEC_DROPPED;
use super::{
    ConditionSchema, CountSchema, DiagnosticSchema, EntrySchema, ExceptionSchema, ExpectedSchema,
    GivenSchema, ImportsSchema, ParamDeclarationSchema, RuleFileSchema, ScenarioSchema,
    SelectorSchema, SpecFileSchema, TemplateFileSchema, ThenSchema,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::entries::types::{Count, ParamName, Value as EntryValue};
use crate::domain::packages::types::{DefinitionKind, DefinitionName, PackageName};
use crate::domain::scalars::types::TextValue;
use crate::domain::scenario_files::types::{
    CountKey, FileKey, ScenarioKeys, ScenarioRoot, Wrapper,
};
use crate::domain::selectors::types::{
    PartSelector, PathPattern, PlacedBySelector, Recursion, RegexText, Selector, Selectors,
    VariableName,
};
use crate::domain::topics::service::{keys, topic};
use crate::domain::topics::types::{Necessity, TopicBody, TopicName};

/// A TOML value, as `value = …` writes it, and what it is read as.
type Input = (&'static str, Reading);
/// The core's value, in debug form.
type Value = String;
type Failure = ();

/// What a row reads its value as.
#[derive(Clone, Copy, Debug)]
enum Reading {
    Selector,
    Count,
    /// Every schema of a definition file against the topic that documents it; the text is ignored.
    Documented,
    /// A `tastemaker.toml` whose text is the value's.
    Workspace,
    /// A lock of one package, written; the text is ignored.
    LockShape,
    /// A `tastemaker.toml` whose text is the value's, answered as `read` or the refusal's text.
    Pin,
    /// A lock whose text is the value's, answered as `read` or the refusal's text.
    Lock,
    /// A param value, as a path or a template reads it.
    Written,
    /// The workspace of a scenario without its own, written for a rule; the text is ignored.
    ScenarioRoot,
    /// The wrapper spec an owned rule is proved through, written; the text is ignored.
    ScenarioWrapper,
    /// A rule whose `[given]` is shared by its two scenarios; the text is ignored.
    SharedGiven,
    /// A rule with the header every rule has, and each scenario that breaks the shape; the text is ignored.
    ScenarioShapes,
}

/// A holder serde reads one value into.
#[derive(serde::Deserialize)]
struct Holder<T> {
    value: T,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_string_is_a_glob"),
        word: Word::Succeeds,
        input: || ("\"src/**\"", Reading::Selector),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_recursive_part_table_is_a_recursive_part"),
        word: Word::Succeeds,
        input: || {
            (
                "{ part = \"use-case\", recursive = true }",
                Reading::Selector,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_range_without_max_is_unbounded"),
        word: Word::Succeeds,
        input: || ("{ min = 1 }", Reading::Count),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("every_key_of_every_definition_file_is_documented"),
        word: Word::Succeeds,
        input: || ("", Reading::Documented),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_lock_is_its_version_then_its_packages_tables_with_nothing_empty"),
        word: Word::Succeeds,
        input: || ("", Reading::LockShape),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_root_that_names_the_tastemaker_that_runs_is_read"),
        word: Word::Succeeds,
        input: || (RUNNING_PIN, Reading::Pin),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_root_that_names_another_tastemaker_says_which_it_needs"),
        word: Word::Succeeds,
        input: || (OTHER_PIN, Reading::Pin),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_local_file_that_names_a_tastemaker_is_refused"),
        word: Word::Succeeds,
        input: || (LOCAL_PIN, Reading::Pin),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_lock_without_its_version_is_refused"),
        word: Word::Succeeds,
        input: || (UNVERSIONED_LOCK, Reading::Lock),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_lock_of_another_version_is_refused"),
        word: Word::Succeeds,
        input: || (OTHER_LOCK, Reading::Lock),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_scenario_without_its_own_workspace_is_placed_by_its_entry_written_whole"),
        word: Word::Succeeds,
        input: || (SCENARIO_ROOT, Reading::ScenarioRoot),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_owned_rule_is_placed_by_a_wrapper_spec"),
        word: Word::Succeeds,
        input: || (SCENARIO_WRAPPER, Reading::ScenarioWrapper),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_definitions_given_is_what_each_scenario_starts_from_its_own_keys_winning",
        ),
        word: Word::Succeeds,
        input: || ("", Reading::SharedGiven),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_scenario_that_breaks_the_shape_is_refused_naming_it_and_the_fix"),
        word: Word::Succeeds,
        input: || ("", Reading::ScenarioShapes),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_text_param_reads_as_written"),
        word: Word::Succeeds,
        input: || ("\"ryo-{lang}\"", Reading::Written),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_whole_param_reads_as_its_number"),
        word: Word::Succeeds,
        input: || ("3", Reading::Written),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_float_param_reads_as_written"),
        word: Word::Succeeds,
        input: || ("1.5", Reading::Written),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_flag_param_reads_as_toml_writes_it"),
        word: Word::Succeeds,
        input: || ("true", Reading::Written),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_list_param_reads_as_toml_writes_it"),
        word: Word::Succeeds,
        input: || ("[\"a\", \"b\"]", Reading::Written),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_list_of_lists_reads_as_toml_writes_it"),
        word: Word::Succeeds,
        input: || ("[[\"app\"], [\"domain\", \"inbound\"]]", Reading::Written),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_table_param_reads_as_toml_writes_it_inline"),
        word: Word::Succeeds,
        input: || {
            (
                "{ resource = \"languages\", path = \"a::B\" }",
                Reading::Written,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_list_of_tables_reads_as_toml_writes_it"),
        word: Word::Succeeds,
        input: || ("[{ a = 1, b = true }]", Reading::Written),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_list_of_scalars_reads_as_toml_writes_it"),
        word: Word::Succeeds,
        input: || ("[1, 2.5, false]", Reading::Written),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_closed_that_writes_the_default_is_refused"),
        word: Word::Refuses,
        input: || ("closed = true\n", Reading::Workspace),
        expect: Expect::error(()),
    },
    Case {
        name: CaseName("a_count_of_text_is_refused"),
        word: Word::Refuses,
        input: || ("\"one\"", Reading::Count),
        expect: Expect::error(()),
    },
];

/// A root naming the Tastemaker that runs.
const RUNNING_PIN: &str = concat!(
    "tastemaker = \"",
    env!("CARGO_PKG_VERSION"),
    "\"\nworkspace = { name = \"w\" }\n"
);
/// A root naming a Tastemaker that never runs here.
const OTHER_PIN: &str = "tastemaker = \"0.0.0-other\"\nworkspace = { name = \"w\" }\n";
/// A local file naming a Tastemaker.
const LOCAL_PIN: &str = "tastemaker = \"0.0.0-other\"\n";
/// The workspace a rule's scenario is placed by: every selector form and a param whose text holds a quote, a
/// line break and a backslash, each written as one TOML value (that text as a multi-line literal string).
const SCENARIO_ROOT: &str = "workspace = { name = \"scenario\", dependencies = [{ package = \"p\", path = \"/packages/p\" }] }\nclosed = false\nrules = [{ rule = \"p/r\", description = \"The definition under test, placed alone.\", include = [\"src/**\", { regex = \"x\" }, { part = \"hex\", recursive = true }, { placed-by = \"e\" }, { variable = \"v\" }], exclude = [\"src/gen/**\"], params = { said = '''\na\nb \"c\" \\''' } }]\n";
/// The wrapper spec an owned rule is placed by.
const SCENARIO_WRAPPER: &str = "description = \"Places rule `r` alone.\"\nwhy = \"Proves rule `r` of spec `s` through its scenario, as its spec places it.\"\nclosed = false\nrules = [{ rule = \"r\", description = \"The rule `r`, placed alone to be proven.\", include = [\"*.rs\"] }]\n";
/// A lock with no `version`.
const UNVERSIONED_LOCK: &str = "dependencies = [\"p\"]\n";
/// A lock of a version no Tastemaker writes.
const OTHER_LOCK: &str = "version = 2\ndependencies = [\"p\"]\n";

/// A rule whose only scenario is `scenario`, written after the header every rule has.
const RULE_HEAD: &str = r#"description = "d"
why = "w"
query = "let x = select * from \"comment\"; emit x subject \"file\" key(\"span\");"

[diagnostic]
primary = "span"
message = "m"
help = "h"

"#;

/// A rule whose `[given]` every scenario starts from: the second scenario writes the shared file again and adds one.
const SHARED_GIVEN: &str = r#"[given]
include = ["src/**"]
files."src/a.txt" = "shared\n"
files."src/b.txt" = "b\n"

[[scenarios]]
id = "the-shared-files"
scenario = "The scenario reads the shared files."

[[scenarios]]
id = "its-own-text"
scenario = "The scenario's own text of a shared file wins, and its own file is added."
given.include = ["**"]
given.files."src/a.txt" = "own\n"
given.files."src/c.txt" = "c\n"
"#;

/// A scenario that breaks the shape, the id it is named by and the fix the refusal says.
const BROKEN_SCENARIOS: [(&str, &str, &str); 3] = [
    (
        "id = \"fails\"\nscenario = \"s.\"\n",
        "fails",
        "not `pass` or `fail`",
    ),
    (
        "id = \"plain-comment\"\nscenario = \" \"\n",
        "plain-comment",
        "`scenario` sentence is empty",
    ),
    (
        "id = \"plain-comment\"\nscenario = \"s.\"\ngiven.at = \"x\"\n",
        "plain-comment",
        "has no `given.at`",
    ),
];

fn expected(input: &str) -> &'static str {
    match input {
        "\"ryo-{lang}\"" => "ryo-{lang}",
        "{ resource = \"languages\", path = \"a::B\" }" => {
            "{ path = \"a::B\", resource = \"languages\" }"
        }
        // Today's bytes, as the domain wrote them before the manifest port did: text and numbers as written, a list
        // and a table as TOML writes them inline.
        "3"
        | "1.5"
        | "true"
        | "[\"a\", \"b\"]"
        | "[[\"app\"], [\"domain\", \"inbound\"]]"
        | "[{ a = 1, b = true }]"
        | "[1, 2.5, false]" => input_text(input),
        "" => "",
        RUNNING_PIN => "read",
        OTHER_PIN => concat!(
            "this repo needs tastemaker 0.0.0-other; ",
            env!("CARGO_PKG_VERSION"),
            " is installed"
        ),
        LOCAL_PIN => {
            "`tastemaker` names the version of Tastemaker a workspace needs: write it in the root file, beside `workspace`"
        }
        SCENARIO_ROOT => SCENARIO_ROOT,
        SCENARIO_WRAPPER => SCENARIO_WRAPPER,
        UNVERSIONED_LOCK | OTHER_LOCK => {
            "a lock begins `version = 1`: run `tm check --fix` to write it again"
        }
        "\"src/**\"" => "Glob(PathPattern(\"src/**\"))",
        "{ min = 1 }" => "Range { min: Amount(1), max: Unbounded }",
        _ => "Part(PartSelector { part: DefinitionName(\"use-case\"), recursion: Recursive })",
    }
}

fn subject((text, reading): Input) -> Result<Value, Failure> {
    let written = format!("value = {text}");
    let answer = match reading {
        Reading::Documented => {
            let mut all = Vec::new();
            for schema in SCHEMAS {
                all.extend(undocumented(*schema)?);
            }
            all.join("\n")
        }
        Reading::Selector => {
            let holder: Holder<SelectorSchema> = toml::from_str(&written).map_err(|_refusal| ())?;
            format!("{:?}", Selector::from(holder.value))
        }
        Reading::Count => {
            let holder: Holder<CountSchema> = toml::from_str(&written).map_err(|_refusal| ())?;
            format!("{:?}", Count::from(holder.value))
        }
        Reading::LockShape => {
            use crate::domain::packages::types::{
                Lock, LockedPackage, LockedRelease, PackageSource, SourcePath, VersionText,
            };
            let lock = Lock {
                dependencies: vec![PackageName("idiomatic.arch".into())],
                packages: vec![LockedPackage {
                    name: PackageName("idiomatic.arch".into()),
                    source: PackageSource::Path(SourcePath("../arch".into())),
                    dependencies: Vec::new(),
                    releases: vec![LockedRelease {
                        kind: DefinitionKind::Rule,
                        name: DefinitionName("file-length".into()),
                        version: VersionText("0.3.0".into()),
                    }],
                }],
            };
            let locked = super::lock_text(&lock).0;
            // The head, the version, then the package by name, its path and its rules by name; no empty list.
            let body = locked
                .lines()
                .filter(|line| !line.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");
            if body
                == "version = 1\ndependencies = [\"idiomatic.arch\"]\n\n[packages.\"idiomatic.arch\"]\npath = \"../arch\"\n\n[packages.\"idiomatic.arch\".rules]\nfile-length = \"0.3.0\""
            {
                String::new()
            } else {
                body
            }
        }
        Reading::Written => {
            let holder: Holder<toml::Value> = toml::from_str(&written).map_err(|_refusal| ())?;
            super::value_text(&super::value(holder.value)).0
        }
        Reading::Pin => match super::workspace_file(
            &crate::outbound::manifest::types::DefinitionText(text.into()),
        ) {
            // The refusal names both versions, which the help line's install needs.
            Err(crate::outbound::manifest::errors::ManifestError::Needs { needs, installed }) => {
                format!("this repo needs tastemaker {needs}; {installed} is installed")
            }
            read => said(read),
        },
        Reading::Lock => match super::lock(&crate::outbound::manifest::types::DefinitionText(
            text.into(),
        )) {
            // A lock of another version is refused with the fix the user runs.
            Err(crate::outbound::manifest::errors::ManifestError::LockVersion { reason }) => reason,
            read => said(read),
        },
        Reading::ScenarioRoot => root_text(),
        Reading::ScenarioWrapper => wrapper_text(),
        Reading::SharedGiven => unmerged_given(),
        Reading::ScenarioShapes => unrefused_scenarios(),
        Reading::Workspace => {
            let read = super::workspace_file(&crate::outbound::manifest::types::DefinitionText(
                text.into(),
            ))
            .map_err(|_refusal| ())?;
            format!("{:?}", read.composition.closed)
        }
    };
    Ok(if answer == expected(text) {
        String::new()
    } else {
        answer
    })
}

/// What each scenario of [`SHARED_GIVEN`] reads when it does not start from the shared `[given]`, its own keys, files and
/// texts winning: nothing when it does.
fn unmerged_given() -> String {
    let Ok(rule) = super::rule(&crate::outbound::manifest::types::DefinitionText(
        format!("{RULE_HEAD}{SHARED_GIVEN}").into(),
    )) else {
        return "the rule is refused".to_owned();
    };
    let said = rule
        .scenarios
        .iter()
        .map(|scenario| {
            let files = scenario
                .files
                .0
                .iter()
                .map(|(path, text)| format!("{}={}", path.0, text.0.trim_end()));
            format!(
                "{:?} {}",
                scenario.include,
                files.collect::<Vec<_>>().join(" ")
            )
        })
        .collect::<Vec<_>>();
    if said
        == [
            r#"Selectors([Glob(PathPattern("src/**"))]) src/a.txt=shared src/b.txt=b"#,
            r#"Selectors([Glob(PathPattern("**"))]) src/a.txt=own src/b.txt=b src/c.txt=c"#,
        ]
    {
        String::new()
    } else {
        said.join("\n")
    }
}

/// The scenarios of [`BROKEN_SCENARIOS`] that are not refused naming the scenario and saying the fix: none when all are.
fn unrefused_scenarios() -> String {
    BROKEN_SCENARIOS
        .iter()
        .filter(|(scenario, id, fix)| {
            let written = format!("{RULE_HEAD}[[scenarios]]\n{scenario}");
            !matches!(
                super::rule(&crate::outbound::manifest::types::DefinitionText(written.into())),
                Err(crate::outbound::manifest::errors::ManifestError::Shape { reason })
                    if reason.contains(&format!("scenario `{id}`")) && reason.contains(fix)
            )
        })
        .map(|(_, id, _)| *id)
        .collect::<Vec<_>>()
        .join(",")
}

/// The keys of a rule's scenario: `include` alone.
fn placing(include: Selectors) -> ScenarioKeys {
    ScenarioKeys {
        include,
        exclude: Selectors::default(),
        file: FileKey::Unnamed,
        count: CountKey::Derived,
        params: std::collections::BTreeMap::new(),
    }
}

/// A glob selector.
fn glob(pattern: &str) -> Selector {
    Selector::Glob(PathPattern(pattern.into()))
}

/// The wrapper spec written for an owned rule's scenario, as text.
fn wrapper_text() -> String {
    super::scenario_wrapper(&Wrapper {
        kind: DefinitionKind::Rule,
        name: &DefinitionName("r".into()),
        spec: &DefinitionName("s".into()),
        keys: &placing(Selectors(vec![glob("*.rs")])),
    })
    .0
    .into_string()
}

/// The workspace written for a shared rule's scenario, as text: every selector form and a param.
fn root_text() -> String {
    super::scenario_root(&ScenarioRoot {
        path: &crate::domain::packages::types::PackageRoot("/packages/p".into()),
        placed: &crate::domain::packages::types::PlacedDefinition {
            kind: DefinitionKind::Rule,
            package: PackageName("p".into()),
            name: DefinitionName("r".into()),
        },
        keys: &ScenarioKeys {
            exclude: Selectors(vec![glob("src/gen/**")]),
            params: std::iter::once((
                ParamName("said".into()),
                EntryValue::Text(TextValue("a\nb \"c\" \\".into())),
            ))
            .collect(),
            ..placing(Selectors(vec![
                glob("src/**"),
                Selector::Regex(RegexText("x".into())),
                Selector::Part(PartSelector {
                    part: DefinitionName("hex".into()),
                    recursion: Recursion::Recursive,
                }),
                Selector::PlacedBy(PlacedBySelector {
                    entry: DefinitionName("e".into()),
                }),
                Selector::Variable(VariableName("v".into())),
            ]))
        },
    })
    .0
    .into_string()
}

/// The text of an input that reads as itself: the `&'static str` the rows are written with.
fn input_text(input: &str) -> &'static str {
    CASES
        .iter()
        .map(|case| (case.input)().0)
        .find(|text| *text == input)
        .unwrap_or_default()
}

/// `read`, or what the refusal says.
fn said<T>(read: Result<T, crate::outbound::manifest::errors::ManifestError>) -> String {
    read.map_or_else(|refusal| refusal.to_string(), |_read| "read".to_owned())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// Why the reading deserializer stopped: it was told the struct's fields, a field it left out was required, or else.
// ignore: idiomatic.arch/crate/thiserror-in-errors - the probe deserializer's stop is test machinery, not a failure of the adapter: it lives beside the probe that raises it
#[derive(Debug, thiserror::Error)]
enum Stop {
    #[error("the fields {0:?}")]
    Fields(&'static [&'static str]),
    #[error("missing `{0}`")]
    Missing(&'static str),
    #[error("{0}")]
    Other(String),
}

// ignore: idiomatic.arch/crate/error-shape - serde's deserializer contract requires its own `de::Error` impl; this is that trait, not a hand-written `std::error::Error`
impl serde::de::Error for Stop {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Other(msg.to_string())
    }

    fn missing_field(field: &'static str) -> Self {
        Self::Missing(field)
    }
}

/// A struct written with every field but the one at `skip`, each holding any value of its type; or, with `names`, only the
/// struct's field names, as serde's derive declares them.
#[derive(Clone, Copy)]
struct Written {
    skip: Option<usize>,
    names: bool,
}

/// Any value of whatever type is asked: zero, empty, absent, or a struct with every field.
struct Anything;

/// The fields of a struct, one entry each, but the one skipped.
struct Entries {
    fields: &'static [&'static str],
    skip: Option<usize>,
    at: usize,
}

impl<'de> serde::de::MapAccess<'de> for Entries {
    type Error = Stop;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Stop>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if Some(self.at) == self.skip {
            self.at += 1;
        }
        let Some(field) = self.fields.get(self.at) else {
            return Ok(None);
        };
        self.at += 1;
        seed.deserialize(serde::de::value::BorrowedStrDeserializer::new(field))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Stop>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(Anything)
    }
}

impl<'de> serde::Deserializer<'de> for Anything {
    type Error = Stop;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bool(false)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i64(0)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u32(0)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_str("")
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_str("")
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_none()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_seq(serde::de::value::SeqDeserializer::<_, Stop>::new(
            std::iter::empty::<bool>(),
        ))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(Entries {
            fields: &[],
            skip: None,
            at: 0,
        })
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(Entries {
            fields,
            skip: None,
            at: 0,
        })
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_enum(serde::de::value::StrDeserializer::<Stop>::new(variants[0]))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i128 u8 u16 u64 u128 f32 f64 char bytes byte_buf unit unit_struct newtype_struct tuple
        tuple_struct identifier ignored_any
    }
}

impl<'de> serde::Deserializer<'de> for Written {
    type Error = Stop;

    fn deserialize_any<V>(self, _: V) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(Stop::Other("a definition file is a struct".to_owned()))
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Stop>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.names {
            return Err(Stop::Fields(fields));
        }
        visitor.visit_map(Entries {
            fields,
            skip: self.skip,
            at: 0,
        })
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit
        unit_struct newtype_struct seq tuple tuple_struct map enum identifier ignored_any
    }
}

/// A key as the reader declares it: its name, and whether the reader refuses a table without it.
type Field = (&'static str, Necessity);

/// Every field of a schema, and whether serde refuses the struct without it.
fn fields<T>() -> Vec<Field>
where
    T: serde::de::DeserializeOwned,
{
    let read = T::deserialize(Written {
        skip: None,
        names: true,
    });
    let names = if let Err(Stop::Fields(names)) = read {
        names
    } else {
        &[]
    };
    names
        .iter()
        .enumerate()
        .map(|(at, name)| {
            let without = T::deserialize(Written {
                skip: Some(at),
                names: false,
            });
            let presence = if matches!(without, Err(Stop::Missing(missing)) if missing == *name) {
                Necessity::Required
            } else {
                Necessity::Optional
            };
            (*name, presence)
        })
        .collect()
}

/// A schema, the topic and table that document it, and the format-2 keys it still reads and no topic documents.
type Documented = (
    fn() -> Vec<Field>,
    &'static str,
    &'static str,
    &'static [&'static str],
);

/// Every schema of a definition file, with its topic's table.
const SCHEMAS: &[Documented] = &[
    (fields::<RuleFileSchema>, "rule", "rule.toml", &[]),
    (fields::<DiagnosticSchema>, "rule", "[diagnostic]", &[]),
    (
        fields::<ParamDeclarationSchema>,
        "rule",
        "[params.<name>]",
        &[],
    ),
    (
        fields::<ExceptionSchema>,
        "rule",
        "[exceptions.<name>]",
        &[],
    ),
    (fields::<ScenarioSchema>, "rule", "[[scenarios]]", &[]),
    (fields::<GivenSchema>, "rule", "[scenarios.given]", &[]),
    (fields::<ThenSchema>, "rule", "[scenarios.then]", &[]),
    (fields::<ExpectedSchema>, "rule", "findings", &[]),
    (fields::<SpecFileSchema>, "spec", "spec.toml", SPEC_DROPPED),
    (
        fields::<EntrySchema>,
        "spec",
        "entry of `rules`, `specs`, `templates`",
        &[],
    ),
    (
        fields::<ConditionSchema>,
        "spec",
        "[conditions.<name>]",
        &["rule"],
    ),
    (fields::<ImportsSchema>, "spec", "imports", &[]),
    (
        fields::<TemplateFileSchema>,
        "template",
        "template.toml",
        &[],
    ),
];

/// Each difference between what the reader declares and what the topic documents: a key one has and the other lacks, or a
/// key each has with another presence.
fn undocumented((read, name, title, format_two): Documented) -> Result<Vec<String>, ()> {
    let topic = topic(TopicName(name)).ok_or(())?;
    let TopicBody::Form(form) = topic.body else {
        return Err(());
    };
    let table = form
        .tables
        .iter()
        .find(|table| table.title.0 == title)
        .ok_or(())?;
    let documented = keys(table)
        .into_iter()
        .map(|key| (key.name.0, key.presence))
        .collect::<Vec<_>>();
    let declared = read()
        .into_iter()
        .filter(|(key, _)| !format_two.contains(key))
        .collect::<Vec<_>>();
    let mut differences = declared
        .iter()
        .filter(|field| !documented.contains(field))
        .map(|(key, presence)| format!("read but not documented as such: `{key}` {presence:?}"))
        .collect::<Vec<_>>();
    differences.extend(
        documented
            .iter()
            .filter(|key| !declared.contains(key))
            .map(|(key, presence)| {
                format!("documented but not read as such: `{key}` {presence:?}")
            }),
    );
    Ok(differences)
}

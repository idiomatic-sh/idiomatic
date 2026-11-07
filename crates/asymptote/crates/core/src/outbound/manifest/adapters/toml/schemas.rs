//! The shape of `asymptote.toml`: each wire shape is a `<Name>Schema`, read by serde, and its checked conversion into
//! the core's type is an `impl TryFrom`.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::{BMF, FIX_KEY, HEADER, PRODUCT, RUN_KEY, SARIF, TIMEOUT};
use crate::domain::captures::service::uncaptured;
use crate::domain::captures::types::CommandKey;
use crate::domain::manifest::service::{hook, known_hooks};
use crate::domain::manifest::types::{
    Argument, Argv, Fix, Group, Hook, HookWord, Manifest, Member, Output, Pathspec, Pin, Seconds,
    Tool, Version,
};
use crate::domain::violations::types::ToolId;

/// The file as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ManifestSchema {
    /// The asymptote version the repository needs, at the workspace root.
    #[serde(default)]
    asymptote: Option<String>,
    /// Its tools.
    #[serde(default)]
    tools: Vec<ToolSchema>,
    /// The globs of its members, when it is a workspace root.
    #[serde(default)]
    members: Vec<String>,
    /// The hooks it maps to groups, when it is the workspace root.
    #[serde(default)]
    hooks: Option<BTreeMap<String, Vec<String>>>,
}

/// One tool as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolSchema {
    /// The name the tool's findings and runs are kept under.
    id: String,
    /// The command line that runs the tool.
    run: Vec<String>,
    /// The command line that fixes what the tool finds, when it has one.
    #[serde(default)]
    fix: Option<Vec<String>>,
    /// The label `check` selects the tool by; required, refused when absent or empty.
    #[serde(default)]
    group: Option<String>,
    /// What the tool writes for asymptote to read: `sarif`, `bmf`, or nothing beyond its exit code.
    #[serde(default)]
    output: Option<String>,
    /// How long the tool may run before it is stopped.
    #[serde(default, rename = "timeout-seconds")]
    timeout_seconds: Option<u64>,
    /// The paths whose contents decide whether an earlier run can be reused.
    #[serde(default)]
    inputs: Option<Vec<String>>,
}

/// Why the file was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum ManifestSchemaError {
    /// It names no tool and no member.
    #[error("it names no tool")]
    NoTools,
    /// A member glob is not one asymptote can expand.
    #[error("member `{0}` is not a relative glob of directories: no `..`, no `**`, no leading `/`")]
    Member(String),
    /// Two tools share an id.
    #[error("two tools are named `{0}`")]
    Duplicate(String),
    /// A tool's id is empty.
    #[error("a tool has an empty id")]
    EmptyId,
    /// A tool's command is empty.
    #[error("tool `{0}` runs an empty command")]
    EmptyRun(String),
    /// A tool's output is not one asymptote reads.
    #[error("tool `{0}`: `output` is `sarif`, `bmf` or absent, not `{1}`")]
    Output(String, String),
    /// A `[hooks]` entry names a hook git does not run.
    #[error("hook `{}` is not a git hook: use one of {}", .0, known_hooks().0)]
    Hook(String),
    /// A `[hooks]` entry lists no group, so its hook would run nothing and pass.
    #[error(
        "hook `{0}` lists no group: list a group, or remove the hook from `[hooks]` to install none"
    )]
    HookEmpty(String),
    /// A tool's `timeout-seconds` is zero, so it would be killed before it ran.
    #[error("tool `{0}`: `timeout-seconds` is a positive number of seconds, not 0")]
    NoTimeout(String),
    /// A tool names no group, or an empty one.
    #[error("tool `{0}`: `group` is required: a label of your choosing, such as `lint`")]
    NoGroup(String),
    /// A tool's command line writes a capture none of its inputs does, so no match gives it a value.
    #[error(
        "tool `{tool}`: `{key}` uses `{{{name}}}`, which no input captures: write it in one of its `inputs`, as in `<directory>/{{{name}}}`"
    )]
    Uncaptured {
        /// The tool.
        tool: String,
        /// The command line, `run` or `fix`.
        key: &'static str,
        /// The capture.
        name: String,
    },
}

/// A written command line as the core's argument vector.
fn argv(arguments: Vec<String>) -> Argv {
    Argv(
        arguments
            .into_iter()
            .map(|argument| Argument(argument.into()))
            .collect(),
    )
}

impl TryFrom<ManifestSchema> for Manifest {
    type Error = ManifestSchemaError;

    fn try_from(schema: ManifestSchema) -> Result<Self, Self::Error> {
        if schema.tools.is_empty() && schema.members.is_empty() && schema.hooks.is_none() {
            return Err(ManifestSchemaError::NoTools);
        }
        let hooks = schema
            .hooks
            .unwrap_or_default()
            .into_iter()
            .map(|(name, words)| {
                let named = hook(&HookWord(name.as_str().into()))
                    .ok_or_else(|| ManifestSchemaError::Hook(name.clone()))?;
                if words.is_empty() {
                    return Err(ManifestSchemaError::HookEmpty(name));
                }
                Ok(Hook {
                    name: named,
                    groups: words.into_iter().map(|word| Group(word.into())).collect(),
                })
            })
            .collect::<Result<Vec<_>, ManifestSchemaError>>()?;
        let members = schema
            .members
            .into_iter()
            .map(|member| {
                let climbs = member.split('/').any(|part| part == ".." || part == "**");
                if member.is_empty() || member.starts_with('/') || climbs {
                    return Err(ManifestSchemaError::Member(member));
                }
                Ok(Member(member.into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = BTreeSet::new();
        let mut tools = Vec::with_capacity(schema.tools.len());
        for tool in schema.tools {
            if tool.id.is_empty() {
                return Err(ManifestSchemaError::EmptyId);
            }
            if !seen.insert(tool.id.clone()) {
                return Err(ManifestSchemaError::Duplicate(tool.id));
            }
            if tool.run.is_empty() {
                return Err(ManifestSchemaError::EmptyRun(tool.id));
            }
            let output = match tool.output.as_deref() {
                None => Output::ExitStatus,
                Some(SARIF) => Output::Sarif,
                Some(BMF) => Output::Bmf,
                Some(other) => {
                    return Err(ManifestSchemaError::Output(
                        tool.id.clone(),
                        other.to_owned(),
                    ));
                }
            };
            if tool.timeout_seconds == Some(0) {
                return Err(ManifestSchemaError::NoTimeout(tool.id));
            }
            let group = match tool.group {
                Some(word) if !word.is_empty() => Group(word.into()),
                Some(_) | None => return Err(ManifestSchemaError::NoGroup(tool.id)),
            };
            let read = Tool {
                id: ToolId(tool.id.into()),
                run: argv(tool.run),
                fix: tool
                    .fix
                    .filter(|fix| !fix.is_empty())
                    .map_or(Fix::Absent, |fix| Fix::Command(argv(fix))),
                output,
                group,
                timeout: Seconds(tool.timeout_seconds.unwrap_or(TIMEOUT)),
                inputs: tool
                    .inputs
                    .unwrap_or_else(|| vec![PRODUCT.to_owned()])
                    .into_iter()
                    .map(|input| Pathspec(input.into()))
                    .collect(),
            };
            if let Some(found) = uncaptured(&read) {
                return Err(ManifestSchemaError::Uncaptured {
                    tool: read.id.0.into(),
                    key: match found.key {
                        CommandKey::Run => RUN_KEY,
                        CommandKey::Fix => FIX_KEY,
                    },
                    name: found.name.0.into(),
                });
            }
            tools.push(read);
        }
        Ok(Self {
            asymptote: schema.asymptote.map_or(Pin::Unpinned, |version| {
                Pin::Pinned(Version(version.into()))
            }),
            tools,
            members,
            hooks,
        })
    }
}

/// A tool as it is written.
#[derive(Debug, serde::Serialize)]
pub(super) struct WrittenToolSchema {
    /// Its id.
    id: String,
    /// Its group.
    group: String,
    /// What runs it.
    run: Vec<String>,
    /// What fixes what it reports, when anything does.
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<Vec<String>>,
    /// What its run is read as, when it is more than an exit status.
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    /// How long it may run.
    #[serde(rename = "timeout-seconds")]
    timeout: u64,
    /// What it reads.
    inputs: Vec<String>,
}

/// A manifest as it is written: what it pins, its members, its hooks, then its tools.
#[derive(Debug, serde::Serialize)]
pub(super) struct WrittenSchema {
    /// The asymptote it pins, when it pins one.
    #[serde(skip_serializing_if = "Option::is_none")]
    asymptote: Option<String>,
    /// Its members, when it is a workspace root.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    members: Vec<String>,
    /// The hooks it maps, when it maps any.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    hooks: BTreeMap<String, Vec<String>>,
    /// Its tools.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<WrittenToolSchema>,
}

impl From<&Manifest> for WrittenSchema {
    fn from(manifest: &Manifest) -> Self {
        let words = |argv: &Argv| argv.0.iter().map(|word| word.0.to_string()).collect();
        Self {
            asymptote: match &manifest.asymptote {
                Pin::Pinned(version) => Some(version.0.to_string()),
                Pin::Unpinned => None,
            },
            members: manifest
                .members
                .iter()
                .map(|member| member.0.to_string())
                .collect(),
            hooks: manifest
                .hooks
                .iter()
                .map(|hook| {
                    (
                        hook.name.0.to_string(),
                        hook.groups
                            .iter()
                            .map(|group| group.0.to_string())
                            .collect(),
                    )
                })
                .collect(),
            tools: manifest
                .tools
                .iter()
                .map(|tool| WrittenToolSchema {
                    id: tool.id.0.to_string(),
                    group: tool.group.0.to_string(),
                    run: words(&tool.run),
                    fix: match &tool.fix {
                        Fix::Command(argv) => Some(words(argv)),
                        Fix::Absent => None,
                    },
                    output: match tool.output {
                        Output::Sarif => Some(SARIF.to_owned()),
                        Output::Bmf => Some(BMF.to_owned()),
                        Output::ExitStatus => None,
                    },
                    timeout: tool.timeout.0,
                    inputs: tool
                        .inputs
                        .iter()
                        .map(|input| input.0.to_string())
                        .collect(),
                })
                .collect(),
        }
    }
}

/// The text of `manifest`, its header first.
pub(super) fn written(manifest: &Manifest) -> Result<String, String> {
    let body =
        toml::to_string(&WrittenSchema::from(manifest)).map_err(|error| error.to_string())?;
    Ok(format!("{HEADER}{body}"))
}

/// The manifest `text` holds.
///
/// # Errors
///
/// The reason, when it is not TOML of the manifest's shape.
pub(super) fn manifest(text: &str) -> Result<Manifest, String> {
    let schema: ManifestSchema =
        toml::from_str(text).map_err(|error| error.message().to_owned())?;
    Manifest::try_from(schema).map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

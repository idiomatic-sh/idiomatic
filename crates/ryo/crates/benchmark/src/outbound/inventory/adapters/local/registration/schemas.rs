//! The registration document: the rules a provider registers and their authority.
use crate::outbound::inventory::adapters::local::types::{CaseText, RegistrationBytes};
use crate::outbound::inventory::types::{
    Command, ExecutablePin, FilePin, IdentityKey, ProviderName, RegisteredModuleName,
    RegisteredRule, Registration, RegistrationAuthority, RuleCommand, RuleDefinition, RuleInstance,
    RuleReason,
};
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    CanonicalJson, ReasonText, RuleName, Sha256Text,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};

/// The generated registration of one provider's rules.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrationSchema {
    authority: AuthoritySchema,
    provider: String,
    defaults: Vec<String>,
    rules: Vec<RegisteredRuleSchema>,
    #[serde(default)]
    identity: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisteredRuleSchema {
    definition: Option<FilePinSchema>,
    modules: BTreeMap<String, FilePinSchema>,
    artifacts: BTreeMap<PathBuf, String>,
    rule_id: String,
    command: Option<CommandSchema>,
    reason: Option<String>,
    #[serde(default)]
    cases: Vec<serde_json::Value>,
    instance: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePinSchema {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutablePinSchema {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum CommandSchema {
    CaseVerification { context: PathBuf },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum AuthoritySchema {
    ReceiptOnly,
    Unavailable {
        reason: String,
    },
    Execution {
        profile: PathBuf,
        profile_sha256: String,
        reference: ExecutablePinSchema,
        worker: ExecutablePinSchema,
    },
}

impl TryFrom<RegistrationSchema> for Registration {
    type Error = Error;

    fn try_from(wire: RegistrationSchema) -> Result<Self, Error> {
        Ok(Self {
            authority: match wire.authority {
                AuthoritySchema::ReceiptOnly => RegistrationAuthority::ReceiptOnly,
                AuthoritySchema::Unavailable { reason } => RegistrationAuthority::Unavailable {
                    reason: ReasonText(reason),
                },
                AuthoritySchema::Execution {
                    profile,
                    profile_sha256,
                    reference,
                    worker,
                } => RegistrationAuthority::Execution {
                    profile,
                    profile_sha256: Sha256Text(profile_sha256),
                    reference: ExecutablePin {
                        path: reference.path,
                        sha256: Sha256Text(reference.sha256),
                    },
                    worker: ExecutablePin {
                        path: worker.path,
                        sha256: Sha256Text(worker.sha256),
                    },
                },
            },
            provider: ProviderName(wire.provider),
            defaults: wire.defaults.into_iter().map(RuleName).collect(),
            rules: wire
                .rules
                .into_iter()
                .map(RegisteredRule::try_from)
                .collect::<Result<_, _>>()?,
            identity: wire
                .identity
                .into_iter()
                .map(|(key, value)| (IdentityKey(key), CanonicalJson(value.to_string())))
                .collect(),
        })
    }
}

impl TryFrom<RegisteredRuleSchema> for RegisteredRule {
    type Error = Error;

    fn try_from(wire: RegisteredRuleSchema) -> Result<Self, Error> {
        let pin = |pin: FilePinSchema| FilePin {
            path: pin.path,
            sha256: Sha256Text(pin.sha256),
        };
        Ok(Self {
            definition: wire.definition.map_or(RuleDefinition::Unpinned, |value| {
                RuleDefinition::Pinned(pin(value))
            }),
            modules: wire
                .modules
                .into_iter()
                .map(|(name, value)| (RegisteredModuleName(name), pin(value)))
                .collect(),
            artifacts: wire
                .artifacts
                .into_iter()
                .map(|(path, digest)| (path, Sha256Text(digest)))
                .collect(),
            rule_id: RuleName(wire.rule_id),
            command: wire.command.map_or(RuleCommand::Unregistered, |command| {
                RuleCommand::Registered(match command {
                    CommandSchema::CaseVerification { context } => {
                        Command::CaseVerification { context }
                    }
                })
            }),
            reason: wire.reason.map_or(RuleReason::Unstated, |reason| {
                RuleReason::Stated(ReasonText(reason))
            }),
            cases: wire
                .cases
                .iter()
                .map(|case| CaseDocument::try_from(CaseText(&case.to_string())))
                .collect::<Result<_, _>>()?,
            instance: wire.instance.map_or(RuleInstance::Undeclared, |value| {
                RuleInstance::Declared(CanonicalJson(value.to_string()))
            }),
        })
    }
}

impl TryFrom<RegistrationBytes<'_>> for Registration {
    type Error = Error;

    fn try_from(bytes: RegistrationBytes<'_>) -> Result<Self, Error> {
        serde_json::from_slice::<RegistrationSchema>(&bytes.0.0)
            .map_err(|error| Error::Json(Box::new(error)))?
            .try_into()
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

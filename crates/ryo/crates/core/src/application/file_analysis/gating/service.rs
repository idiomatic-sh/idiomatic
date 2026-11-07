//! Which rules a file's applicability lets run, and the relations those rules read.
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution as _;
use crate::application::file_analysis::constants::RULE_SKIPPED_APPLICABILITY_METADATA_UNAVAILABLE;
use crate::application::file_analysis::constants::RULE_SKIPPED_SYNTAX_ACCEPTANCE_UNDETERMINED;
use crate::domain::workers::service::governs_rule;
use crate::domain::workers::types::Gating;
use crate::domain::workers::types::Tables;
use ryo_execution::inbound::use_cases::execution::interface::Demand;
use ryo_execution::inbound::use_cases::metadata::interface::Governing;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase as _;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::runs::types::RuleNumber;
use std::borrow::Cow;
use super::super::service::FileAnalysis;
use super::super::types::{RequestVisitSetting, RuleGate};

impl FileAnalysis {
    /// The parsed relations needed by rules that govern this path and whose applicability is met.
    pub(in crate::application::file_analysis) fn active_wanted<'a>(
        tables: &Tables,
        setting: &'a RequestVisitSetting<'_>,
        acceptance: SyntaxAcceptance,
    ) -> Cow<'a, [Relation]> {
        if setting.metadata.is_empty() {
            return Cow::Borrowed(setting.wanted);
        }
        // All rules are active for legacy plans, and complete metadata plus accepted syntax
        // activates every policy. Keep the caller's relation slice in both cases.
        if setting.applicability_can_gate == Gating::NeverGates
            || acceptance == SyntaxAcceptance::Accepted
                && setting.applicability.len() == setting.metadata.len()
        {
            return Cow::Borrowed(setting.wanted);
        }
        let demanded = setting
            .program
            .demanded_rules(|rule| {
                if governs_rule(&MetadataUseCaseImpl, tables, rule) == Governing::Governs {
                    Demand::Demanded
                } else {
                    Demand::Undemanded
                }
            });
        Cow::Owned(
            setting
                .wanted
                .iter()
                .copied()
                .filter(|relation| {
                    setting.metadata.iter().enumerate().any(|(place, _)| {
                        demanded.get(place).copied() == Some(Demand::Demanded)
                            && match setting.applicability.get(place).copied() {
                                Some(RuleApplicability::Always) => true,
                                Some(RuleApplicability::AcceptedSyntax) => {
                                    acceptance == SyntaxAcceptance::Accepted
                                }
                                None => false,
                            }
                            && setting.needs.get(place).is_some_and(|needs| {
                                needs.reads.iter().any(|read| read.relation == *relation)
                            })
                    })
                })
                .collect(),
        )
    }

    /// Whether this setting contains a policy that can change a file's active rules.
    pub(in crate::application::file_analysis) fn may_gate_rules(setting: &RequestVisitSetting<'_>, acceptance: SyntaxAcceptance) -> Gating {
        if setting.applicability_can_gate == Gating::MayGate
            && !(acceptance == SyntaxAcceptance::Accepted
                && setting.applicability.len() == setting.metadata.len())
        {
            Gating::MayGate
        } else {
            Gating::NeverGates
        }
    }

    /// Removes rules whose declared syntax applicability is not met and records why each one was
    /// skipped for this file. The skip is separate from relation partiality: it says the rule was
    /// not applicable, not that its input was read incompletely.
    pub(in crate::application::file_analysis) fn gate_rules(
        tables: &mut Tables,
        setting: &RequestVisitSetting<'_>,
        file: FileId,
        acceptance: SyntaxAcceptance,
    ) -> Vec<RuleGate> {
        let demanded = setting
            .program
            .demanded_rules(|rule| {
                if governs_rule(&MetadataUseCaseImpl, tables, rule) == Governing::Governs {
                    Demand::Demanded
                } else {
                    Demand::Undemanded
                }
            });
        let mut skipped = vec![RuleGate::Open; setting.metadata.len()];
        for (place, blocked) in skipped.iter_mut().enumerate() {
            if demanded.get(place).copied() != Some(Demand::Demanded) {
                continue;
            }
            let applicability = setting.applicability.get(place).copied();
            *blocked = match applicability {
                Some(RuleApplicability::Always) => RuleGate::Open,
                Some(RuleApplicability::AcceptedSyntax) => {
                    if acceptance == SyntaxAcceptance::Accepted {
                        RuleGate::Open
                    } else {
                        RuleGate::Blocked
                    }
                }
                None => RuleGate::Blocked,
            };
            if *blocked == RuleGate::Open {
                continue;
            }
            MetadataUseCaseImpl.unplace(
                &mut tables.mask,
                RuleNumber(u32::try_from(place).unwrap_or(u32::MAX)),
            );
            let reason = match (applicability, acceptance) {
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Rejected) => {
                    let (Some(rule), Some(path)) = (
                        setting.ids.get(place),
                        setting
                            .paths
                            .get(usize::try_from(file.0).unwrap_or(usize::MAX)),
                    ) else {
                        continue;
                    };
                    tables
                        .skipped
                        .push(ryo_values::domain::runs::types::Skipped {
                            rule: rule.clone(),
                            path: path.clone(),
                            reason: ryo_values::domain::runs::types::SkipReason::SyntaxRejected,
                        });
                    continue;
                }
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Undetermined) => {
                    RULE_SKIPPED_SYNTAX_ACCEPTANCE_UNDETERMINED
                }
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Accepted)
                | (Some(RuleApplicability::Always), _) => continue,
                (None, _) => RULE_SKIPPED_APPLICABILITY_METADATA_UNAVAILABLE,
            };
            let Some(rule) = setting.ids.get(place) else {
                continue;
            };
            let Some(path) = setting
                .paths
                .get(usize::try_from(file.0).unwrap_or(usize::MAX))
            else {
                continue;
            };
            tables.unknowns.push(
                ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                    rule: rule.clone(),
                    path: path.clone(),
                    message: ryo_execution::inbound::use_cases::metadata::interface::Message(
                        reason.into(),
                    ),
                },
            );
        }
        skipped
    }
}

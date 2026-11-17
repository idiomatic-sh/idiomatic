//! The `findings` service: free functions named by verb over findings, with no IO and no ports.

use std::collections::BTreeMap;

use super::constants::CHECK_SEPARATOR;
use super::types::{
    ByteOffset, CheckChoice, CheckName, Column, Finding, FindingRule, FirstLine, Help, Identifying,
    IdentityParts, Line, MessageTemplate, MessageText, Ordinal, Place, Placeholder,
    PlaceholderValue, Region, RenderedText, Report, RuleId, RuleText, Sameness, SourceText,
    Verdict,
};
use crate::domain::paths::types::RepoPath;

/// One of an entry's built-in checks: the entry, and the check.
#[must_use]
pub fn check_id(identity: impl std::fmt::Display, check: impl std::fmt::Display) -> FindingRule {
    FindingRule {
        rule: RuleId(identity.to_string().into()),
        check: CheckChoice::Named(CheckName(check.to_string().into())),
    }
}

/// What a finding of the rule `rule` is of: the rule itself.
#[must_use]
pub const fn of_rule(rule: RuleId) -> FindingRule {
    FindingRule {
        rule,
        check: CheckChoice::Rule,
    }
}

/// What a finding is of, as it is printed and as an ignore or a scenario names it: the rule, or `entry#check`.
#[must_use]
pub fn printed(rule: &FindingRule) -> RuleText {
    match &rule.check {
        CheckChoice::Rule => RuleText(rule.rule.0.clone()),
        CheckChoice::Named(check) => {
            RuleText(format!("{}{CHECK_SEPARATOR}{}", rule.rule.0, check.0).into())
        }
    }
}

/// A finding about a whole file, with no help.
#[must_use]
pub fn error(rule: FindingRule, path: RepoPath, message: impl Into<MessageText>) -> Finding {
    finding(rule, path, Place::Whole, message, Help::Silent)
}

/// A finding.
#[must_use]
pub fn finding(
    rule: FindingRule,
    path: RepoPath,
    place: Place,
    message: impl Into<MessageText>,
    help: Help,
) -> Finding {
    Finding {
        path,
        place,
        rule,
        message: message.into(),
        help,
        fingerprint: super::types::Fingerprint::default(),
    }
}

/// The lines and columns the byte range `start..end` of `text` covers; none when either lies outside it.
#[must_use]
pub fn spanning(text: SourceText<'_>, start: ByteOffset, end: ByteOffset) -> Option<Region> {
    let position = |offset: ByteOffset| {
        let before = text.0.get(..offset.0)?;
        let line = before.matches('\n').count() + 1;
        let column = before
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count()
            + 1;
        // A line or column past what 32 bits count saturates.
        Some((
            Line(u32::try_from(line).unwrap_or(u32::MAX)),
            Column(u32::try_from(column).unwrap_or(u32::MAX)),
        ))
    };
    let (start_line, start_column) = position(start)?;
    let (end_line, end_column) = position(end)?;
    Some(Region {
        start_line,
        start_column,
        end_line,
        end_column,
    })
}

/// Places a byte range using the file's already indexed line starts. Columns count Unicode scalars.
#[must_use]
pub fn indexed_span(
    source: crate::domain::files::types::SourceView<'_>,
    start: ByteOffset,
    end: ByteOffset,
) -> Option<Region> {
    let position = |offset: ByteOffset| {
        // This also rejects offsets in the middle of a UTF-8 code point.
        let _: &str = source.text.get(..offset.0)?;
        let offset64 = u64::try_from(offset.0).unwrap_or(u64::MAX);
        let after = source
            .starts
            .partition_point(|line_start| *line_start <= offset64);
        let from = after
            .checked_sub(1)
            .and_then(|index| source.starts.get(index));
        let from = usize::try_from(from.copied().unwrap_or_default()).unwrap_or(usize::MAX);
        let column = source.text.get(from..offset.0)?.chars().count() + 1;
        // A line or column past what 32 bits count saturates.
        Some((
            Line(u32::try_from(after.max(1)).unwrap_or(u32::MAX)),
            Column(u32::try_from(column).unwrap_or(u32::MAX)),
        ))
    };
    let (start_line, start_column) = position(start)?;
    let (end_line, end_column) = position(end)?;
    Some(Region {
        start_line,
        start_column,
        end_line,
        end_column,
    })
}

/// Whether two findings report the same thing: the same file, place, rule and message, whatever their help.
#[must_use]
pub fn repeats(one: &Finding, other: &Finding) -> Sameness {
    if one.path == other.path
        && one.place == other.place
        && one.rule == other.rule
        && one.message == other.message
    {
        Sameness::Same
    } else {
        Sameness::Different
    }
}

/// `findings` sorted, each reported once.
pub fn settle(findings: &mut Vec<Finding>) {
    findings.sort();
    findings.dedup_by(|later, kept| repeats(later, kept) == Sameness::Same);
}

/// What each finding's fingerprint hashes, in order: its rule and file, and the trimmed text of its first line of code
/// (`first_line` answers it; empty without a region), and how many findings with the same parts came before. The message
/// is left out, so a value it measures changing never renames the finding.
pub fn identities<'a>(
    findings: impl IntoIterator<Item = &'a Finding>,
    first_line: impl Fn(&Finding) -> Option<FirstLine>,
) -> Vec<Identifying> {
    identified(findings.into_iter().map(|finding| {
        let line = match finding.place {
            Place::Whole => FirstLine::default(),
            Place::At(_) | Place::Line(_) => first_line(finding).unwrap_or_default(),
        };
        super::types::PreparedIdentity { finding, line }
    }))
}

/// Assigns ordinals in the final global finding order after independent line extraction.
pub fn identified<'a>(
    prepared: impl IntoIterator<Item = super::types::PreparedIdentity<'a>>,
) -> Vec<Identifying> {
    let mut seen: BTreeMap<IdentityParts, Ordinal> = BTreeMap::new();
    prepared
        .into_iter()
        .map(|super::types::PreparedIdentity { finding, line }| {
            let parts = IdentityParts([
                printed(&finding.rule).0,
                finding.path.0.clone(),
                line.0.trim().into(),
            ]);
            let ordinal = seen.entry(parts.clone()).or_insert(Ordinal(0));
            let identifying = Identifying {
                parts,
                ordinal: *ordinal,
            };
            ordinal.0 += 1;
            identifying
        })
        .collect()
}

/// What a ratchet identifies a finding about a whole file or directory by: its rule and path alone, so its record
/// survives the value its message measures changing.
#[must_use]
pub fn whole_identity(finding: &Finding) -> Identifying {
    Identifying {
        parts: IdentityParts([
            printed(&finding.rule).0,
            finding.path.0.clone(),
            Box::default(),
        ]),
        ordinal: Ordinal(0),
    }
}

/// Failed when there is any finding: every finding is an error.
#[must_use]
pub fn verdict(report: &Report) -> Verdict {
    if report.findings.is_empty() {
        Verdict::Passed
    } else {
        Verdict::Failed
    }
}

/// `template` with each `{column}` replaced by its value in `values`; a brace naming no value is kept as written.
#[must_use]
pub fn render(
    template: &MessageTemplate,
    values: &BTreeMap<Placeholder<'_>, PlaceholderValue>,
) -> RenderedText {
    let mut rendered = String::with_capacity(template.0.len());
    let mut rest: &str = &template.0;
    while let Some((before, after)) = rest.split_once('{') {
        rendered.push_str(before);
        let named = after
            .split_once('}')
            .and_then(|(name, tail)| values.get(&Placeholder(name)).map(|value| (value, tail)));
        if let Some((value, tail)) = named {
            rendered.push_str(&value.0);
            rest = tail;
        } else {
            rendered.push('{');
            rest = after;
        }
    }
    rendered.push_str(rest);
    RenderedText(rendered)
}

/// The help a rule's finding carries: its rendered `help`.
#[must_use]
pub fn said(text: impl Into<MessageText>) -> Help {
    Help::Said(text.into())
}

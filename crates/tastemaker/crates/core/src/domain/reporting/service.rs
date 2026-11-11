//! Pure report settlement and identity construction over already supplied values.
use super::constants::{DROPPED, EMPTY, SEPARATOR, UNANSWERED, UNREAD};
use super::types::{Blindness, Candidate, Fingerprinted, IncomingText, Standing};
use crate::domain::applications::service::readable_shape;
use crate::domain::applications::types::{Compared, Plan};
use crate::domain::contracts::types::{Contract, Holding};
use crate::domain::definitions::types::Exactness;
use crate::domain::files::types::BorrowedText;
use crate::domain::findings::constants::{IGNORE, TASTEMAKER, TEMPLATE, UNKNOWN};
use crate::domain::findings::service::{check_id, error, finding, whole_identity};
use crate::domain::findings::types::{
    Finding, Fingerprint, FirstLine, Help, Identifying, Place, Report, Suppression,
};
use crate::domain::paths::types::RepoPath;

/// Completes the workspace report after every finding has its identity.
#[must_use]
pub fn report_of(
    plan: &Plan,
    Fingerprinted { standing, accepted }: Fingerprinted,
    sourced: std::collections::BTreeMap<RepoPath, Fingerprint>,
) -> Report {
    let mut evaluated = plan.evaluated.clone();
    let _: bool = evaluated.insert(check_id(TASTEMAKER, IGNORE));
    Report {
        workspace: plan.workspace.clone(),
        findings: standing,
        suppressed: accepted,
        evaluated,
        sourced,
    }
}

/// What a finding is before its file's ignores are applied, as the one text a fingerprint hashes: its rule, place, message and help,
/// and, for one placed by a stretch of bytes, the stretch.
fn incoming_text(
    finding: &Finding,
    span: Option<&crate::domain::comments::types::ByteSpan>,
) -> IncomingText {
    let place = match finding.place {
        Place::Whole => String::new(),
        Place::Line(line) => line.0.to_string(),
        Place::At(at) => format!(
            "{}.{}-{}.{}",
            at.start_line.0, at.start_column.0, at.end_line.0, at.end_column.0
        ),
    };
    let span = span.map_or_else(String::new, |each| {
        format!("{}-{}", each.start.0, each.end.0)
    });
    IncomingText(
        [
            &*crate::domain::findings::service::printed(&finding.rule).0,
            &*finding.message.0,
            &place,
            &span,
            match &finding.help {
                Help::Said(said) => &*said.0,
                Help::Silent => EMPTY,
            },
        ]
        .join(SEPARATOR)
        .into(),
    )
}

/// What each file named by `findings` or `spans` has before its ignores are applied, as one fingerprint: the findings it has, in
/// a fixed order, each as `incoming_text` writes it, hashed one after the other. A file that only holds an ignore has the empty
/// chain, which `empty` is the fingerprint of.
#[must_use]
pub fn incoming_of(
    findings: &[Finding],
    spans: &[crate::domain::applications::types::SpannedFinding],
    ignoring: &[&RepoPath],
    fingerprint: &dyn Fn(&Identifying) -> Fingerprint,
) -> std::collections::BTreeMap<RepoPath, Fingerprint> {
    let mut texts: std::collections::BTreeMap<RepoPath, Vec<IncomingText>> =
        std::collections::BTreeMap::new();
    for finding in findings {
        texts
            .entry(finding.path.clone())
            .or_default()
            .push(incoming_text(finding, None));
    }
    for spanned in spans {
        texts
            .entry(spanned.finding.path.clone())
            .or_default()
            .push(incoming_text(&spanned.finding, Some(&spanned.span)));
    }
    for path in ignoring {
        let _: &mut Vec<IncomingText> = texts.entry((*path).clone()).or_default();
    }
    texts
        .into_iter()
        .map(|(path, mut each)| {
            each.sort();
            let chained = each.iter().fold(Fingerprint(0), |before, text| {
                fingerprint(&Identifying {
                    parts: crate::domain::findings::types::IdentityParts([
                        before.0.to_string().into(),
                        text.0.clone(),
                        Box::default(),
                    ]),
                    ordinal: crate::domain::findings::types::Ordinal(0),
                })
            });
            (path, chained)
        })
        .collect()
}

/// The findings the ignores kept and those they accepted, together and sorted: suppressed findings are fingerprinted with
/// the rest, as they would be unignored.
#[must_use]
pub fn candidates(kept: Vec<Finding>, suppressed: Vec<Suppression>) -> Vec<Candidate> {
    let mut all = kept
        .into_iter()
        .map(|finding| Candidate {
            finding,
            line: FirstLine::default(),
            acceptance: Standing::Stands,
        })
        .chain(suppressed.into_iter().map(|suppression| Candidate {
            finding: suppression.finding,
            line: FirstLine::default(),
            acceptance: Standing::Accepted(suppression.reason),
        }))
        .collect::<Vec<_>>();
    all.sort_by(|left, right| left.finding.cmp(&right.finding));
    all
}

/// Each finding with its fingerprint, from its identity when it is whole and from the lines it starts on otherwise,
/// standing or, with the reason it is ignored, accepted.
#[must_use]
pub fn fingerprinted(
    all: Vec<Candidate>,
    identified: Vec<Identifying>,
    fingerprint: &dyn Fn(&Identifying) -> Fingerprint,
) -> Fingerprinted {
    let mut standing = Vec::new();
    let mut accepted = Vec::new();
    for (
        Candidate {
            mut finding,
            acceptance,
            ..
        },
        identifying,
    ) in all.into_iter().zip(identified)
    {
        finding.fingerprint = match finding.place {
            Place::Whole => fingerprint(&whole_identity(&finding)),
            Place::At(_) | Place::Line(_) => fingerprint(&identifying),
        };
        match acceptance {
            Standing::Accepted(reason) => accepted.push(Suppression { finding, reason }),
            Standing::Stands => standing.push(finding),
        }
    }
    Fingerprinted { standing, accepted }
}

/// Apply comments and capture identities while one source view is resident.
#[must_use]
pub fn prepared_file(
    plan: &Plan,
    path: &RepoPath,
    findings: Vec<Finding>,
    comments: Option<&Vec<crate::domain::comments::types::Comment>>,
    view: Option<crate::domain::files::types::SourceView<'_>>,
) -> Vec<Candidate> {
    use crate::domain::comments::service::{code_region, indexed_lines};
    use crate::domain::findings::types::SourceText;
    use crate::domain::ignores::service::{first_line, suppress};
    use crate::domain::ignores::types::Suppressed;
    let lines = view
        .zip(comments)
        .map(|(view, found)| indexed_lines(view, found));
    let suppressed = match (view, comments, lines.as_ref()) {
        (Some(view), Some(found), Some(lines)) => suppress(
            findings,
            path,
            SourceText(view.text.0),
            found,
            lines,
            &plan.known,
        ),
        _ => Suppressed {
            kept: findings,
            suppressed: Vec::new(),
        },
    };
    let mut all = candidates(suppressed.kept, suppressed.suppressed);
    for candidate in &mut all {
        candidate.line = first_line(&candidate.finding, view, lines.as_ref()).unwrap_or_default();
        if let (Place::At(region), Some(view), Some(lines)) =
            (candidate.finding.place, view, lines.as_ref())
        {
            candidate.finding.place =
                Place::At(code_region(lines, SourceText(view.text.0), region));
        }
    }
    all
}

/// The `#unknown` of a contract the comparison cannot read: its file or its content is one no linked language reads, a
/// row of it was dropped, or its non-blank content answered no relation the comparison reads.
fn blind(contract: &Contract, reason: Blindness) -> Finding {
    let reason = match reason {
        Blindness::Unread => UNREAD,
        Blindness::Dropped => DROPPED,
        Blindness::Unanswered => UNANSWERED,
    };
    finding(
        check_id(&contract.identity, UNKNOWN),
        contract.target.clone(),
        Place::Whole,
        format!(
            "the `{}` template is not compared with `{}`: {reason}",
            contract.identity, contract.target.0
        ),
        Help::Silent,
    )
}

/// The lines of `text` that are not blank, each without its trailing white space.
fn written_lines(text: BorrowedText<'_>) -> Vec<BorrowedText<'_>> {
    text.0
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(BorrowedText)
        .collect()
}

/// Whether `text` holds the rendered content of `contract` as text: each of its lines that is not blank, in order,
/// with others between them, or, for an exact template, exactly its lines that are not blank.
fn holds_text(contract: &Contract, text: BorrowedText<'_>) -> Holding {
    let wanted = written_lines(BorrowedText(&contract.text.0));
    let written = written_lines(text);
    let held = if contract.exact == Exactness::Contains {
        let mut rest = written.iter();
        wanted.iter().all(|line| rest.any(|made| made.0 == line.0))
    } else {
        wanted
            .iter()
            .map(|line| line.0)
            .eq(written.iter().map(|line| line.0))
    };
    if held { Holding::Holds } else { Holding::Apart }
}

/// The first line at which a verbatim file departs from its content, as a finding.
fn departure(contract: &Contract, text: BorrowedText<'_>) -> Option<Finding> {
    if text.0 == &*contract.text.0 {
        return None;
    }
    let line = text
        .0
        .lines()
        .zip(contract.text.0.lines())
        .position(|(found, expected)| found != expected)
        .unwrap_or_else(|| text.0.lines().count().min(contract.text.0.lines().count()))
        + 1;
    Some(error(
        check_id(&contract.identity, TEMPLATE),
        contract.target.clone(),
        format!(
            "the file departs from the verbatim `{}` template at line {line}",
            contract.identity.0
        ),
    ))
}

/// Compare one placed contract with its supplied source, through the rows of the shared relations both answered: a
/// data document by its keys and elements, an exact document block by block in order, code by its declarations. A
/// verbatim content is also its text, byte for byte, which needs no language; any other contract the comparison cannot
/// read is `#unknown`, never a pass.
#[must_use]
pub fn mismatch(
    contract: &Contract,
    compared: &Compared<'_>,
    text: BorrowedText<'_>,
) -> Vec<Finding> {
    use crate::domain::blocks::service::{blocks, sequence};
    use crate::domain::contracts::service::{compare, facts, message};
    use crate::domain::contracts::types::{ComparedText, Shape};
    use crate::domain::declarations::service::declarations;
    use crate::domain::definitions::constants::{VARIABLES_KEY, WORKSPACE_FILE};
    use crate::domain::keyed::service::compare_data;
    use crate::domain::paths::service::name;
    let Compared {
        data,
        unread,
        dropped,
    } = compared;
    let template = |shown: &dyn std::fmt::Display| {
        error(
            check_id(&contract.identity, TEMPLATE),
            contract.target.clone(),
            shown.to_string(),
        )
    };
    let verbatim = contract.exact == Exactness::Verbatim;
    let rows = |path: &RepoPath| data.get(path).map_or(&[][..], Vec::as_slice);
    let expected = facts(rows(&contract.rendered));
    let found = facts(rows(&contract.target));
    let Some(shape) = readable_shape(contract, compared, &expected) else {
        if verbatim {
            return departure(contract, text).into_iter().collect();
        }
        let reason = if dropped.contains(&contract.target) || dropped.contains(&contract.rendered) {
            Blindness::Dropped
        } else if unread.contains(&contract.target) || unread.contains(&contract.rendered) {
            Blindness::Unread
        } else {
            Blindness::Unanswered
        };
        // A file the comparison cannot read is met when it holds its template's text, line by line in order (all of
        // it and no more for an exact template); else it is `#unknown`, never a pass.
        if reason != Blindness::Dropped && holds_text(contract, text) == Holding::Holds {
            return Vec::new();
        }
        return vec![blind(contract, reason)];
    };
    let mismatches = match shape {
        Shape::Document => {
            return sequence(
                &blocks(&expected, ComparedText(&contract.text.0)),
                &blocks(&found, ComparedText(text.0)),
                &contract.identity,
            )
            .into_iter()
            .map(|said| template(&said))
            .chain(departure(contract, text).filter(|_| verbatim))
            .collect();
        }
        Shape::Data => {
            // A manifest's variables are the instance's values: its own may hold more than its template's.
            let instance_values =
                (&*name(&contract.target).0 == WORKSPACE_FILE).then_some(VARIABLES_KEY);
            compare_data(&expected, &found, contract.exact, instance_values)
        }
        Shape::Code | Shape::Blind => compare(
            &declarations(&expected),
            &declarations(&found),
            contract.exact,
        ),
    };
    if mismatches.is_empty() && verbatim {
        // The declarations are the same but the file is not the content: a verbatim template places its text whole, so the
        // first line that departs is what the finding says.
        return departure(contract, text).into_iter().collect();
    }
    mismatches
        .into_iter()
        .map(|mismatch| template(&message(&mismatch, &contract.identity)))
        .collect::<Vec<_>>()
}

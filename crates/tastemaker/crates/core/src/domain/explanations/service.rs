//! The `explanations` service: free functions named by verb that write what `explain` says as sections of Markdown lines, from
//! what the workspace's files and definitions hold, with no IO and no ports.

use super::constants::{
    DECLARERS, FILE_MANY, FILE_ONE, HERE, LINE_BREAK, PARAMS, PLACEMENT, QUERY_CLOSE, QUERY_OPEN,
    RULE_MANY, RULE_ONE, RULES, SCAFFOLD, SCOPES, SHAPE, SKIPPED, SKIPPED_NOTE, UNPLACED, UNSAID,
};
use super::types::{Page, Section, SectionLine, SectionTitle, Tally, Totals};
use crate::domain::definitions::types::{Definition, RuleFile, SpecFile, TemplateFile, Why};
use crate::domain::entries::types::{Described, Description, ParamDeclarations};
use crate::domain::files::types::SkippedDirectory;
use crate::domain::findings::types::RuleId;
use crate::domain::findings::types::WorkspaceName;
use crate::domain::paths::types::Relative;
use crate::domain::scopes::types::{Identity, Scope};

/// A section of `title` holding `lines`.
fn section(title: SectionTitle, lines: Vec<SectionLine>) -> Section {
    Section { title, lines }
}

/// A scope as a line: its identity and its directory, the workspace's own shown as `.`.
#[must_use]
pub fn scope_line(scope: &Scope) -> SectionLine {
    let dir = if scope.dir.0.is_empty() {
        HERE
    } else {
        &scope.dir.0
    };
    SectionLine(format!("- `{}` at `{dir}`", scope.identity.0).into())
}

/// The command that writes what the entry of `identity` places at `at`, a directory relative to where `explain` ran:
/// the line to copy, which writes only the files that are missing.
#[must_use]
pub fn scaffold_line(identity: &Identity, at: Relative<'_>) -> SectionLine {
    let at = if at.0.is_empty() { HERE } else { at.0 };
    SectionLine(format!("- scaffolded by `{SCAFFOLD} {} --at {at}`", identity.0).into())
}

/// What places a path: the template entry of `identity`, or none.
#[must_use]
pub fn placed_by(identity: Option<&Identity>) -> Vec<SectionLine> {
    vec![match identity {
        Some(entry) => SectionLine(format!("- placed by `{}`", entry.0).into()),
        None => SectionLine(UNPLACED.into()),
    }]
}

/// A rule that reads a path, as a line: its id.
#[must_use]
pub fn rule_line(rule: &RuleId) -> SectionLine {
    SectionLine(format!("- `{}`", rule.0).into())
}

/// What applies to a path: the scopes it lies in, what places it and the rules that read it.
#[must_use]
pub fn path_sections(
    scopes: Vec<SectionLine>,
    placed: Vec<SectionLine>,
    rules: Vec<SectionLine>,
) -> Vec<Section> {
    vec![
        section(SectionTitle(SCOPES.into()), scopes),
        section(SectionTitle(PLACEMENT.into()), placed),
        section(SectionTitle(RULES.into()), rules),
    ]
}

/// A definition's params, one line each: its name, its type and what it means.
#[must_use]
pub(crate) fn param_lines(params: &ParamDeclarations) -> Vec<SectionLine> {
    params
        .0
        .iter()
        .map(|(param, declaration)| {
            let said = match &declaration.description {
                Described::Said(text) => &*text.0,
                Described::Unsaid => UNSAID,
            };
            SectionLine(format!("- `{}` ({}): {said}", param.0, declaration.kind.0).into())
        })
        .collect()
}

/// What a rule holds: how many cases prove it, and its query.
#[must_use]
pub(crate) fn rule_shape(rule: &RuleFile) -> Vec<SectionLine> {
    vec![
        SectionLine(format!("{} scenario(s)", rule.scenarios.len()).into()),
        SectionLine(QUERY_OPEN.into()),
        SectionLine(rule.query.0.trim_end().into()),
        SectionLine(QUERY_CLOSE.into()),
    ]
}

/// What a spec holds: the templates, rules and specs it places.
#[must_use]
pub(crate) fn spec_shape(spec: &SpecFile) -> Vec<SectionLine> {
    let templates = spec.composition.templates.iter().map(|entry| {
        SectionLine(format!("- template `{}` at `{}`", entry.template.0, entry.file.0).into())
    });
    let rules = spec
        .composition
        .rules
        .iter()
        .map(|entry| SectionLine(format!("- rule `{}`", entry.rule.0).into()));
    let specs = spec
        .composition
        .specs
        .iter()
        .map(|entry| SectionLine(format!("- spec `{}`", entry.spec.0).into()));
    templates.chain(rules).chain(specs).collect()
}

/// What a template holds: how many cases prove it.
#[must_use]
pub(crate) fn template_shape(template: &TemplateFile) -> Vec<SectionLine> {
    vec![SectionLine(
        format!("{} scenario(s)", template.scenarios.len()).into(),
    )]
}

/// What a definition is, of whichever kind it is: its title with what it says and why, its params, and what it holds.
#[must_use]
pub fn definition_explained(title: SectionTitle, definition: &Definition) -> Vec<Section> {
    match definition {
        Definition::Rule(rule) => definition_sections(
            title,
            &rule.description,
            &rule.why,
            &rule.params,
            rule_shape(rule),
        ),
        Definition::Spec(spec) => definition_sections(
            title,
            &spec.description,
            &spec.why,
            &spec.params,
            spec_shape(spec),
        ),
        Definition::Template(template) => definition_sections(
            title,
            &template.description,
            &template.why,
            &template.params,
            template_shape(template),
        ),
    }
}

/// What a definition is: its title with what it says and why, its params, and what it holds.
#[must_use]
pub(crate) fn definition_sections(
    title: SectionTitle,
    description: &Description,
    why: &Why,
    params: &ParamDeclarations,
    shape: Vec<SectionLine>,
) -> Vec<Section> {
    vec![
        Section {
            title,
            lines: vec![
                SectionLine(description.0.clone()),
                SectionLine(why.0.clone()),
            ],
        },
        section(SectionTitle(PARAMS.into()), param_lines(params)),
        section(SectionTitle(SHAPE.into()), shape),
    ]
}

/// A skipped directory as a line: its name and who declares it.
#[must_use]
pub(crate) fn skipped_line(skipped: &SkippedDirectory) -> SectionLine {
    let by = skipped
        .by
        .iter()
        .map(|declarer| &*declarer.0)
        .collect::<Vec<_>>()
        .join(DECLARERS);
    SectionLine(format!("- `{}`: {by}", skipped.name.0).into())
}

/// What the workspace is: its name with what it governs, places and lints, its scopes, and the directories no walk enters.
#[must_use]
pub fn workspace_sections(
    name: &WorkspaceName,
    totals: Totals,
    scopes: &[Scope],
    skipped: &[SkippedDirectory],
) -> Vec<Section> {
    let Totals {
        governed: Tally(governed),
        placed: Tally(placed),
        rules: Tally(rules),
    } = totals;
    vec![
        Section {
            title: SectionTitle(name.0.clone()),
            lines: vec![SectionLine(
                format!(
                    "{governed} {} governed, {placed} placed, {rules} {} in the lint call",
                    if governed == 1 { FILE_ONE } else { FILE_MANY },
                    if rules == 1 { RULE_ONE } else { RULE_MANY },
                )
                .into(),
            )],
        },
        section(
            SectionTitle(SCOPES.into()),
            scopes.iter().map(scope_line).collect(),
        ),
        section(
            SectionTitle(SKIPPED.into()),
            std::iter::once(SectionLine(SKIPPED_NOTE.into()))
                .chain(skipped.iter().map(skipped_line))
                .collect(),
        ),
    ]
}

/// The sections as one Markdown page: each heading, then its lines.
#[must_use]
pub fn page(sections: &[Section]) -> Page {
    Page(
        sections
            .iter()
            .map(|each| {
                let lines = each
                    .lines
                    .iter()
                    .map(|line| &*line.0)
                    .collect::<Vec<_>>()
                    .join(LINE_BREAK);
                format!("## {}\n\n{lines}\n", each.title.0)
            })
            .collect::<Vec<_>>()
            .join(LINE_BREAK)
            .into(),
    )
}

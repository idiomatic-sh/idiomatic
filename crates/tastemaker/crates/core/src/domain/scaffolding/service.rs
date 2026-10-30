// ignore: idiomatic.arch/file-length - scaffolding is one concept: a placed file's name, its first render context and the text written, each a free function over the same template entry; the discards `unused_results` asks for, which rustfmt writes over several lines, took it past the limit.
//! The `scaffolding` service: free functions named by verb that name the file an entry places at an instance and build the
//! context its template is rendered with when the file is first written, with no IO and no ports.

use std::collections::BTreeMap;

use std::collections::BTreeSet;

use super::constants::{EXTENSION_MARK, GROUP_OPEN};
use super::types::{
    Declaring, FirstWriting, LocalSplit, MissingParam, Naming, Optionality, PlacedName, Reads,
    Unfilled, Waiting, Waits,
};
use crate::domain::captures::service::{
    captured_names, escaped, pieces, substitute_pattern, unescaped, wildcards, written_piece,
};
use crate::domain::captures::types::{
    CaptureBinding, CaptureName, CaptureValue, Captures, PatternPiece, PlainText, Template,
    Wildcards,
};
use crate::domain::contexts::constants::{EXTENSION, SCAFFOLD, SCAFFOLDING};
use crate::domain::contexts::types::{ContextKey, ContextText, RenderContext};
use crate::domain::definitions::service::content;
use crate::domain::definitions::types::{LocalPackage, PackagePath, SpecFile, TemplateFile};
use crate::domain::entries::service::{interpolate, minimum};
use crate::domain::entries::types::{
    Amount, Composition, Count, ParamDeclarations, ParamSource, ParamValue, ParamValues, Setting,
    SpecEntry, TemplateEntry, Value,
};
use crate::domain::paths::service::{containment, join, name};
use crate::domain::paths::types::{Containment, PathText, RepoPath};
use crate::domain::selectors::types::{PathPattern, Selector};

/// The fewest files of an entry an instance needs whatever its files hold: a condition's fewer answer, so a file only one
/// answer asks for waits until `check` decides its condition.
#[must_use]
pub fn fewest(count: &Count) -> Amount {
    match count {
        Count::When {
            then, otherwise, ..
        } => (*then).min(*otherwise),
        Count::Exactly(_) | Count::Range { .. } => minimum(count).unwrap_or_default(),
    }
}

/// The values set on the command line, as the captures a pattern's `{name}`s are replaced by.
#[must_use]
pub fn given(set: &BTreeMap<Setting, Setting>) -> Captures {
    Captures(
        set.iter()
            .map(|(name, value)| (CaptureName(name.0.clone()), CaptureValue(value.0.clone())))
            .collect(),
    )
}

/// Each capture's name with its value, as a pattern is substituted with them.
pub(crate) fn bindings(captures: &Captures) -> impl Iterator<Item = CaptureBinding<'_>> {
    captures
        .0
        .iter()
        .map(|(name, value)| CaptureBinding { name, value })
}

/// The name of the file `file` places with `captures`: its `{name}`s replaced, and a single `*` the extension given as `ext`.
#[must_use]
pub fn placed_name(file: &PathPattern, captures: &Captures) -> PlacedName {
    let named = substitute_pattern(Template(&file.0), bindings(captures)).0;
    let found = pieces(Template(&named));
    let one_star = found
        .iter()
        .filter(|piece| **piece == PatternPiece::Star)
        .count()
        == 1
        && !found.contains(&PatternPiece::Question);
    PlacedName(match captures.0.get(&CaptureName(EXTENSION.into())) {
        Some(extension) if one_star => found
            .into_iter()
            .map(|piece| match piece {
                PatternPiece::Star => escaped(PlainText(&extension.0)).0,
                other @ (PatternPiece::Literal(_)
                | PatternPiece::Question
                | PatternPiece::Capture(_)
                | PatternPiece::Span(_)) => written_piece(other).0,
            })
            .collect(),
        _ => named,
    })
}

/// The first capture a placed name still holds, which nothing gave: `x` of `items/{x}.txt` or of `{*x}`.
#[must_use]
pub fn unfilled(placed: &PlacedName) -> Option<Unfilled> {
    captured_names(Template(&placed.0))
        .into_iter()
        .next()
        .map(|name| Unfilled(name.0.into()))
}

/// Whether a placed name is one file or still a pattern.
#[must_use]
pub fn naming(placed: &PlacedName) -> Naming {
    if wildcards(Template(&placed.0)) == Wildcards::Held {
        Naming::Pattern
    } else {
        Naming::Path
    }
}

/// The literal params an entry passes, their `{name}`s replaced by `captures`.
#[must_use]
pub fn literal_values(
    params: &ParamValues,
    captures: &Captures,
    written: &dyn crate::domain::entries::types::ValueWriter,
) -> Captures {
    let pairs = bindings(captures).collect::<Vec<_>>();
    Captures(
        params
            .0
            .iter()
            .filter_map(|(param, value)| match value {
                ParamValue::Literal(value) => Some((
                    CaptureName(param.0.clone()),
                    CaptureValue(
                        written
                            .write(&interpolate(value, &pairs))
                            .0
                            .into_boxed_str(),
                    ),
                )),
                ParamValue::Param(_) | ParamValue::Variable(_) => None,
            })
            .collect(),
    )
}

/// The params `declared` takes from where an instance is: the name of its directory, or the workspace's name.
#[must_use]
pub fn named_by_directory(
    declared: &ParamDeclarations,
    directory: &RepoPath,
    workspace: &crate::domain::findings::types::WorkspaceName,
) -> Captures {
    Captures(
        declared
            .0
            .iter()
            .filter_map(|(param, declaration)| {
                let value = match declaration.source {
                    ParamSource::DirectoryName => name(directory).0,
                    ParamSource::WorkspaceName => workspace.0.clone(),
                    ParamSource::Given => return None,
                };
                Some((CaptureName(param.0.clone()), CaptureValue(value)))
            })
            .collect(),
    )
}

/// What a template declaring `declared` is rendered with when its file at `directory` is first written: the values given, each as a capture the
/// other files of the instance see, the params the entry passes or names, and those the directory names.
///
/// # Errors
///
/// The first param the template declares that nothing gave.
pub fn first_render(
    captures: &Captures,
    declared: &ParamDeclarations,
    passed: &ParamValues,
    directory: &RepoPath,
    written: &dyn crate::domain::entries::types::ValueWriter,
) -> Result<RenderContext, MissingParam> {
    let mut context = RenderContext::default();
    // A template says what a file must declare for the check, and what a file starts with when it is written: the part that only a
    // first writing carries is under `{% if scaffold | default(false) %}`.
    drop(
        context
            .values
            .insert(ContextKey(SCAFFOLD.into()), ContextText(SCAFFOLDING.into())),
    );
    for (capture, value) in &captures.0 {
        drop(
            context
                .values
                .insert(ContextKey(capture.0.clone()), ContextText(value.0.clone())),
        );
        // What a capture takes in the files already placed is what `captures.<name>` lists, so a value given for a capture is
        // the one the other files of the instance see.
        drop(context.captured.insert(
            ContextKey(capture.0.clone()),
            vec![ContextText(value.0.clone())],
        ));
    }
    let pairs = bindings(captures).collect::<Vec<_>>();
    for (param, declaration) in &declared.0 {
        match passed.0.get(param) {
            Some(ParamValue::Literal(value)) => {
                drop(context.values.insert(
                    ContextKey(param.0.clone()),
                    ContextText(written.write(&interpolate(value, &pairs)).0.into()),
                ));
            }
            Some(ParamValue::Param(reference)) => {
                if let Some(value) = captures.0.get(&CaptureName(reference.0.clone())) {
                    drop(
                        context
                            .values
                            .insert(ContextKey(param.0.clone()), ContextText(value.0.clone())),
                    );
                }
            }
            Some(ParamValue::Variable(_)) | None => {
                if declaration.source == ParamSource::DirectoryName
                    && !context.values.contains_key(&ContextKey(param.0.clone()))
                {
                    drop(
                        context
                            .values
                            .insert(ContextKey(param.0.clone()), ContextText(name(directory).0)),
                    );
                }
            }
        }
    }
    // A param the template declares and nothing gave is named, with the flag that gives it, instead of failing in the renderer.
    match declared
        .0
        .keys()
        .find(|param| !context.values.contains_key(&ContextKey(param.0.clone())))
    {
        Some(missing) => Err(MissingParam(missing.0.to_string())),
        None => Ok(context),
    }
}

/// Splits the files a workspace governs from those of the packages it depends on that sit in the repository: a package whose
/// directory holds one of the files is local, and its files are its own.
#[must_use]
pub fn local_split(files: Vec<RepoPath>, packages: Vec<PackagePath>) -> LocalSplit {
    let packages = packages
        .into_iter()
        .filter(|local| {
            files
                .iter()
                .any(|file| containment(file, &local.path) == Containment::Inside)
        })
        .map(|local| LocalPackage {
            package: local.name,
            directory: local.path,
        })
        .collect::<Vec<_>>();
    let governed = files
        .into_iter()
        .filter(|file| {
            !packages
                .iter()
                .any(|package| containment(file, &package.directory) == Containment::Inside)
        })
        .collect();
    LocalSplit { packages, governed }
}

/// Whether an entry's files are optional: the fewest it asks for is none.
#[must_use]
pub fn optionality(count: &Count) -> Optionality {
    if fewest(count) == Amount(0) {
        Optionality::Optional
    } else {
        Optionality::Required
    }
}

/// Whether `template` says what a first writing of its file starts it with: its content reads the scaffold flag. An optional
/// file is written only when it does.
#[must_use]
pub fn first_writing(template: Option<&TemplateFile>) -> FirstWriting {
    if template.is_some_and(|template| content(template).0.contains(SCAFFOLD)) {
        FirstWriting::Said
    } else {
        FirstWriting::Unsaid
    }
}

/// The `{name}`s a pattern or a placed name holds, each without its `*`, as the captures they name: an escaped brace
/// names none.
fn braced(text: Template<'_>) -> impl Iterator<Item = CaptureName> {
    captured_names(text).into_iter()
}

/// The captures the optional files among `templates` introduce.
#[must_use]
pub fn waited(templates: &[TemplateEntry]) -> Waits {
    Waits(
        templates
            .iter()
            .filter(|entry| optionality(&entry.count) == Optionality::Optional)
            .flat_map(|entry| braced(Template(&entry.file.0)))
            .collect::<BTreeSet<_>>(),
    )
}

/// Whether `placed` still names a capture an optional file introduces, and so waits for that file.
#[must_use]
pub fn waiting(placed: &PlacedName, waits: &Waits) -> Waiting {
    if braced(Template(&placed.0)).any(|name| waits.0.contains(&name)) {
        Waiting::Waits
    } else {
        Waiting::Ready
    }
}

/// Whether `path` is the module file that declares the instance at `at`: the file beside its directory named as it is,
/// with the extension of whatever language the module is written in (`src/inbound` is declared by `src/inbound.<ext>`).
#[must_use]
fn declares(path: &RepoPath, at: &RepoPath) -> Declaring {
    let declared = path
        .0
        .strip_prefix(&*at.0)
        .and_then(|rest| rest.strip_prefix(EXTENSION_MARK))
        .is_some_and(|extension| !extension.is_empty() && !extension.contains('/'));
    if declared {
        Declaring::Declares
    } else {
        Declaring::Other
    }
}

/// Whether `entry`, under `base`, places the module file of the instance at `at` with the instance's `captures`: a
/// required entry whose file is the module file.
#[must_use]
pub fn declaring(
    base: &RepoPath,
    entry: &TemplateEntry,
    captures: &Captures,
    at: &RepoPath,
) -> Declaring {
    let placed = placed_name(&entry.file, captures);
    if optionality(&entry.count) == Optionality::Required
        && join(base, PathText(&unescaped(Template(&placed.0)).0))
            .is_some_and(|path| declares(&path, at) == Declaring::Declares)
    {
        Declaring::Declares
    } else {
        Declaring::Other
    }
}

/// The names the params an entry passes read: the param a `{ param = "x" }` names, and the captures a literal holds.
fn passed_reads(passed: &ParamValues) -> impl Iterator<Item = CaptureName> + '_ {
    passed.0.values().flat_map(|value| match value {
        ParamValue::Param(reference) => vec![CaptureName(reference.0.clone())],
        ParamValue::Literal(literal) => texts(literal)
            .into_iter()
            .flat_map(|text| braced(text).collect::<Vec<_>>())
            .collect(),
        ParamValue::Variable(_) => Vec::new(),
    })
}

/// Every text a value holds, through its lists and tables, in order, as the template its `{name}`s are read from.
fn texts(value: &Value) -> Vec<Template<'_>> {
    match value {
        Value::Text(text) => vec![Template(&text.0)],
        Value::List(items) => items.iter().flat_map(texts).collect(),
        Value::Table(table) => table.values().flat_map(texts).collect(),
        Value::Whole(_) | Value::Float(_) | Value::Flag(_) => Vec::new(),
    }
}

/// The names a template entry reads from what is set: the captures its file holds, `ext` for a file named with a `*`,
/// the params its template declares, and the names the params it passes read.
#[must_use]
pub fn template_reads(entry: &TemplateEntry, template: Option<&TemplateFile>) -> Reads {
    let mut reads = braced(Template(&entry.file.0)).collect::<BTreeSet<_>>();
    if pieces(Template(&entry.file.0)).contains(&PatternPiece::Star) {
        let _: bool = reads.insert(CaptureName(EXTENSION.into()));
    }
    reads.extend(passed_reads(&entry.header.params));
    if let Some(template) = template {
        reads.extend(
            template
                .params
                .0
                .keys()
                .map(|param| CaptureName(param.0.clone())),
        );
    }
    Reads(reads)
}

/// The names a spec entry reads from what is set before its files do: the captures its `include` holds, the params its
/// spec declares, and the names the params it passes read.
#[must_use]
pub fn spec_reads(entry: &SpecEntry, spec: &SpecFile) -> Reads {
    let mut reads = entry
        .header
        .include
        .0
        .iter()
        .flat_map(|selector| match selector {
            Selector::Glob(pattern) => braced(Template(&pattern.0)).collect(),
            // A regex captures by its named groups, `(?P<name>…)` or `(?<name>…)`.
            Selector::Regex(regex) => regex
                .0
                .split(GROUP_OPEN)
                .skip(1)
                .filter_map(|group| group.strip_prefix('P').unwrap_or(group).strip_prefix('<'))
                .filter_map(|group| group.split_once('>'))
                .map(|(name, _)| CaptureName(name.into()))
                .collect(),
            Selector::Part(_) | Selector::PlacedBy(_) | Selector::Variable(_) => Vec::new(),
        })
        .collect::<BTreeSet<_>>();
    reads.extend(passed_reads(&entry.header.params));
    reads.extend(
        spec.params
            .0
            .keys()
            .map(|param| CaptureName(param.0.clone())),
    );
    Reads(reads)
}

/// The names the conditions of `composition` read from what is set: those the params their rules are passed read.
#[must_use]
pub fn condition_reads(composition: &Composition) -> Reads {
    Reads(
        composition
            .conditions
            .values()
            .flat_map(|condition| passed_reads(&condition.entry.header.params).collect::<Vec<_>>())
            .collect(),
    )
}

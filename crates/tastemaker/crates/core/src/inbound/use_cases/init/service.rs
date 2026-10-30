//! The `init` service: local free-function helpers, private to this use case.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{StringBody, TopKind};
use crate::domain::captures::service::escaped;
use crate::domain::captures::types::{CaptureName, PlainText};
use crate::domain::definitions::constants::{ASYMPTOTE_DIRECTORY, ASYMPTOTE_FILE, WORKSPACE_FILE};
use crate::domain::files::types::FileText;
use crate::domain::packages::constants::{
    INIT_CAPTURE, INIT_CAPTURE_FALLBACK, INIT_CAPTURE_JOIN, INIT_DIRECTORY_ENTRY, INIT_ENTRIES,
    INIT_FILE_ENTRY, INIT_PATH, INIT_UNICODE_ESCAPE, INIT_WORKSPACE, INITIAL_FILES,
};
use crate::domain::paths::types::{PathText, RepoPath};

/// The closed workspace file `init` writes for a project holding `files`: one `unchecked` entry per path at its top, a
/// directory's with a capture name of its own, so nothing the project already holds is a `#closed` finding and anything
/// added after is. The paths `init` writes itself get none, `asymptote.toml` and `.asymptote/` have an optional entry of their own, and the walk never lists the store a check keeps.
pub(super) fn workspace_text(files: &[RepoPath]) -> FileText {
    let own = INITIAL_FILES
        .iter()
        .map(|(file, _)| top(PathText(file)))
        .chain([
            top(PathText(WORKSPACE_FILE)),
            top(PathText(ASYMPTOTE_FILE)),
            top(PathText(ASYMPTOTE_DIRECTORY)),
        ])
        .collect::<BTreeSet<_>>();
    let mut tops: BTreeMap<RepoPath, TopKind> = BTreeMap::new();
    for file in files {
        let name = top(PathText(&file.0));
        let kind = if file.0.contains('/') {
            TopKind::Directory
        } else {
            TopKind::File
        };
        if !name.0.is_empty() && !own.contains(&name) {
            let found = tops.entry(name).or_insert(kind);
            if kind == TopKind::Directory {
                *found = TopKind::Directory;
            }
        }
    }
    let mut captures = BTreeSet::new();
    let entries = tops
        .iter()
        .map(|(name, kind)| {
            // The name as the path pattern writes it, so a `*`, `?` or brace in it is the name, never a wildcard or a capture.
            let pattern = RepoPath(escaped(PlainText(&name.0)).0.into());
            // The capture is put in first and the name last: a name is never read for a placeholder (`<capture>/`).
            match kind {
                TopKind::Directory => INIT_DIRECTORY_ENTRY
                    .replace(INIT_CAPTURE, &capture(name, &mut captures).0)
                    .replace(INIT_PATH, &in_string(&pattern).0),
                TopKind::File => INIT_FILE_ENTRY.replace(INIT_PATH, &in_string(&pattern).0),
            }
        })
        .collect::<Vec<_>>()
        .concat();
    FileText(INIT_WORKSPACE.replace(INIT_ENTRIES, &entries).into())
}

/// `name` as it is written inside a TOML basic string: a backslash, a quote and a control character escaped, so a name
/// that holds one is still the one string it names, never text that breaks the file.
fn in_string(name: &RepoPath) -> StringBody {
    let mut written = String::with_capacity(name.0.len());
    for character in name.0.chars() {
        match character {
            '\\' | '"' => {
                written.push('\\');
                written.push(character);
            }
            control if control.is_control() => {
                // `\u` and the code point in four hexadecimal digits: a control character is below U+00A0.
                let code = u32::from(control);
                written.push_str(INIT_UNICODE_ESCAPE);
                for shift in [12_u32, 8, 4, 0] {
                    written.push(char::from_digit((code >> shift) & 0xF, 16).unwrap_or('0'));
                }
            }
            other => written.push(other),
        }
    }
    StringBody(written)
}

/// The first segment of `path`: the path at the project's top it lies in.
fn top(path: PathText<'_>) -> RepoPath {
    RepoPath(path.0.split('/').next().unwrap_or_default().into())
}

/// A capture name for the directory `name`, unique among `taken`: its letters and digits, the rest joined by `-`, and a
/// number after it when another directory took it first.
fn capture(name: &RepoPath, taken: &mut BTreeSet<CaptureName>) -> CaptureName {
    let words = name
        .0
        .split(|character| !char::is_ascii_alphanumeric(&character))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(INIT_CAPTURE_JOIN)
        .to_ascii_lowercase();
    let base = if words.is_empty() {
        INIT_CAPTURE_FALLBACK.to_owned()
    } else {
        words
    };
    let mut chosen = CaptureName(base.clone().into());
    let mut number = 2_usize;
    while !taken.insert(chosen.clone()) {
        chosen = CaptureName(format!("{base}{INIT_CAPTURE_JOIN}{number}").into());
        number += 1;
    }
    chosen
}

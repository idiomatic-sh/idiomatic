//! The manifest port, implemented with `toml`: the file is found by walking up from the start, and read whole.

use std::path::{Path, PathBuf};

use globset::GlobBuilder;

use super::constants::{
    CURRENT, FILE, INPUTS_JOIN, INSTALLED, NESTED, NESTED_HOOKS, NO_DIRECTORY, NO_MANIFEST,
    NOT_TEXT, SPACE, TWICE, TWICE_END, UNBOUND, UNBOUND_INPUTS, UNDECLARED,
};
use super::schemas::{manifest, written};
use crate::domain::captures::service::{bind, captured, instance, pieces, substitute, unbound};
use crate::domain::captures::types::{Captures, Written};
use crate::domain::manifest::service::{pinned, repeated};
use crate::domain::manifest::types::{
    Group, Manifest, Member, Pathspec, Pin, ProductRoot, Satisfaction, Tool, Version,
};
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::{HookTable, Products};

/// The TOML manifest adapter.
#[derive(Debug)]
pub(crate) struct ManifestTomlAdapter;

/// The directories `member`, relative to `root`, names: each segment without a glob character is joined as it is,
/// and each with one lists the directories at that depth that match it.
fn expand(root: &Path, member: &Member) -> Result<Vec<PathBuf>, ManifestError> {
    let mut found = vec![root.to_path_buf()];
    for segment in member
        .0
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != CURRENT)
    {
        if segment.contains(['*', '?', '[', '{']) {
            let matcher = GlobBuilder::new(segment)
                .literal_separator(true)
                .build()
                .map_err(|error| ManifestError::Member {
                    member: member.0.to_string(),
                    reason: error.kind().to_string(),
                })?
                .compile_matcher();
            let mut next = Vec::new();
            for directory in &found {
                let entries = match std::fs::read_dir(directory) {
                    Ok(entries) => entries,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => {
                        return Err(ManifestError::Unreadable {
                            path: directory.display().to_string(),
                            reason: error.to_string(),
                        });
                    }
                };
                for entry in entries.flatten() {
                    let named = entry.file_name();
                    if entry.path().is_dir() && matcher.is_match(Path::new(&named)) {
                        next.push(entry.path());
                    }
                }
            }
            found = next;
        } else {
            found = found
                .iter()
                .map(|directory| directory.join(segment))
                .collect();
        }
    }
    found.retain(|path| path.is_dir());
    found.sort();
    Ok(found)
}

/// The captures that make `input`, relative to `root`, name an entry on disk, each beside `bound`: one segment at a
/// time, a segment with a capture `bound` gives no value matched against each entry of the directories reached so
/// far, sorted by name. `bound` alone when `input` writes no capture it lacks.
fn matching(
    root: &Path,
    input: &Pathspec,
    bound: &Captures,
) -> Result<Vec<Captures>, ManifestError> {
    if unbound(Written(&input.0), bound).is_empty() {
        return Ok(vec![bound.clone()]);
    }
    let mut reached = vec![(root.to_path_buf(), bound.clone())];
    for written in input
        .0
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != CURRENT)
    {
        let mut next = Vec::new();
        for (directory, captures) in reached {
            if unbound(Written(written), &captures).is_empty() {
                let segment = substitute(Written(written), &captures).0;
                next.push((directory.join(segment), captures));
                continue;
            }
            let entries = match std::fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(ManifestError::Unreadable {
                        path: directory.display().to_string(),
                        reason: error.to_string(),
                    });
                }
            };
            // An entry the directory cannot list, or a name that is no text, is an error, never a capture fewer.
            let mut names: Vec<String> = Vec::new();
            for entry in entries {
                let name = entry
                    .map_err(|error| ManifestError::Unreadable {
                        path: directory.display().to_string(),
                        reason: error.to_string(),
                    })?
                    .file_name()
                    .into_string()
                    .map_err(|name| ManifestError::Unreadable {
                        path: directory.display().to_string(),
                        reason: format!("{NOT_TEXT}{}", name.to_string_lossy()),
                    })?;
                names.push(name);
            }
            names.sort();
            let pattern = pieces(Written(written));
            for name in names {
                if let Some(more) = bind(&pattern, Written(&name), &captures) {
                    next.push((directory.join(&name), more));
                }
            }
        }
        reached = next;
    }
    // Only an entry that is not there is absent: one whose metadata cannot be read is an error.
    let mut found = Vec::with_capacity(reached.len());
    for (path, captures) in reached {
        match std::fs::metadata(&path) {
            Ok(_) => found.push(captures),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(ManifestError::Unreadable {
                    path: path.display().to_string(),
                    reason: error.to_string(),
                });
            }
        }
    }
    Ok(found)
}

/// Every set of values `inputs` capture on disk at `root`, in the order the inputs and their entries give them: each
/// input that writes a capture the inputs before it did not extends each set they found by the entries it matches,
/// a capture they bound taking its value; an input that writes no new capture keeps every set.
fn bindings(root: &Path, inputs: &[Pathspec]) -> Result<Vec<Captures>, ManifestError> {
    let mut found = vec![Captures::default()];
    for input in inputs {
        let mut next: Vec<Captures> = Vec::new();
        for bound in &found {
            for each in matching(root, input, bound)? {
                if !next.contains(&each) {
                    next.push(each);
                }
            }
        }
        found = next;
    }
    Ok(found)
}

/// `read`'s tools, each whose inputs write a capture expanded into one tool per set of values they capture on disk at
/// `root`, in their place: a directory that appears is a tool more, and one that goes is a tool less.
fn expanded(root: &Path, path: &Path, read: Manifest) -> Result<Manifest, ManifestError> {
    let mut tools: Vec<Tool> = Vec::with_capacity(read.tools.len());
    for tool in read.tools {
        let order = captured(&tool.inputs);
        if order.is_empty() {
            tools.push(tool);
            continue;
        }
        let bound = bindings(root, &tool.inputs)?;
        // A captured tool whose inputs match nothing on disk is refused, naming them: a mistyped input never drops it.
        if bound.is_empty() {
            return Err(ManifestError::Shape {
                path: path.display().to_string(),
                reason: [
                    UNBOUND,
                    &*tool.id.0,
                    UNBOUND_INPUTS,
                    &tool
                        .inputs
                        .iter()
                        .map(|input| &*input.0)
                        .collect::<Vec<_>>()
                        .join(INPUTS_JOIN),
                ]
                .concat(),
            });
        }
        for captures in bound {
            tools.push(instance(&tool, &order, &captures));
        }
    }
    if let Some(twice) = repeated(&tools) {
        return Err(ManifestError::Shape {
            path: path.display().to_string(),
            reason: [TWICE, &*twice.0, TWICE_END].concat(),
        });
    }
    Ok(Manifest { tools, ..read })
}

impl ManifestPort for ManifestTomlAdapter {
    fn find(&self, start: &ProductRoot) -> Result<ProductRoot, ManifestError> {
        let found = Path::new(&*start.0)
            .ancestors()
            .find(|directory| directory.join(FILE).is_file())
            .map(|directory| ProductRoot(directory.display().to_string().into()))
            .ok_or_else(|| ManifestError::Missing {
                start: start.0.to_string(),
            })?;
        // Every command finds its product first: one the workspace root pins to another asymptote stops here, before
        // anything is read or written.
        let top = Path::new(&*found.0)
            .ancestors()
            .filter(|directory| directory.join(FILE).is_file())
            .last()
            .map_or_else(
                || found.clone(),
                |top| ProductRoot(top.display().to_string().into()),
            );
        if let Pin::Pinned(needed) = self.read(&top)?.asymptote
            && pinned(&needed, &Version(INSTALLED.into())) == Satisfaction::Unmet
        {
            return Err(ManifestError::Pinned {
                needed: needed.0.to_string(),
                installed: INSTALLED.to_owned(),
            });
        }
        Ok(found)
    }

    fn read(&self, root: &ProductRoot) -> Result<Manifest, ManifestError> {
        let path = Path::new(&*root.0).join(FILE);
        let text = std::fs::read_to_string(&path).map_err(|error| ManifestError::Unreadable {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        let read = manifest(&text).map_err(|reason| ManifestError::Shape {
            path: path.display().to_string(),
            reason,
        })?;
        expanded(Path::new(&*root.0), &path, read)
    }

    fn members(&self, root: &ProductRoot) -> Result<Products, ManifestError> {
        let base = Path::new(&*root.0);
        let manifest = self.read(root)?;
        if manifest.members.is_empty() {
            return Ok(Products::default());
        }
        if base
            .ancestors()
            .skip(1)
            .any(|directory| directory.join(FILE).is_file())
        {
            return Err(ManifestError::NotRoot {
                path: base.join(FILE).display().to_string(),
            });
        }
        let mut members = std::collections::BTreeSet::new();
        for member in &manifest.members {
            let directories = expand(base, member)?;
            if directories.is_empty() {
                return Err(ManifestError::Member {
                    member: member.0.to_string(),
                    reason: NO_DIRECTORY.to_owned(),
                });
            }
            members.extend(directories);
        }
        let mut roots = Vec::with_capacity(members.len());
        for directory in members {
            let shown = directory
                .strip_prefix(base)
                .unwrap_or(&directory)
                .display()
                .to_string();
            let member = ProductRoot(directory.display().to_string().into());
            if !directory.join(FILE).is_file() {
                return Err(ManifestError::Member {
                    member: shown,
                    reason: NO_MANIFEST.to_owned(),
                });
            }
            let read = self.read(&member)?;
            if !read.members.is_empty() {
                return Err(ManifestError::Member {
                    member: shown,
                    reason: NESTED.to_owned(),
                });
            }
            if !read.hooks.is_empty() {
                return Err(ManifestError::Member {
                    member: shown,
                    reason: NESTED_HOOKS.to_owned(),
                });
            }
            roots.push(member);
        }
        Ok(Products(roots))
    }

    fn topmost(&self, start: &ProductRoot) -> Result<ProductRoot, ManifestError> {
        let nearest = self.find(start)?;
        let top = Path::new(&*nearest.0)
            .ancestors()
            .filter(|directory| directory.join(FILE).is_file())
            .last()
            .map_or_else(|| nearest.0.to_string(), |top| top.display().to_string());
        Ok(ProductRoot(top.into()))
    }

    fn hooks(&self, start: &ProductRoot) -> Result<HookTable, ManifestError> {
        let root = self.topmost(start)?;
        let manifest = self.read(&root)?;
        let mapped = manifest.hooks.clone();
        let mut declared: Vec<Group> = manifest
            .tools
            .iter()
            .map(|tool| tool.group.clone())
            .collect();
        for member in self.members(&root)?.0 {
            declared.extend(
                self.read(&member)?
                    .tools
                    .iter()
                    .map(|tool| tool.group.clone()),
            );
        }
        if declared.is_empty() {
            return Ok(HookTable(mapped));
        }
        for each in &mapped {
            if let Some(missing) = each.groups.iter().find(|group| !declared.contains(group)) {
                return Err(ManifestError::Hook {
                    hook: each.name.0.to_string(),
                    reason: [UNDECLARED, &*missing.0].join(SPACE),
                });
            }
        }
        Ok(HookTable(mapped))
    }

    fn create(&self, root: &ProductRoot, manifest: &Manifest) -> Result<(), ManifestError> {
        let path = Path::new(&*root.0).join(FILE);
        let text = written(manifest).map_err(|reason| ManifestError::Shape {
            path: path.display().to_string(),
            reason,
        })?;
        let unwritable = |error: std::io::Error| ManifestError::Unwritable {
            path: path.display().to_string(),
            reason: error.to_string(),
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(unwritable)?;
        std::io::Write::write_all(&mut file, text.as_bytes()).map_err(unwritable)
    }
}

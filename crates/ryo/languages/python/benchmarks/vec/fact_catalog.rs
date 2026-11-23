//! redo-fills pass 2 (THROWAWAY, bead ryo-fmx3.1): the facts this prototype provider states beyond its relations catalog,
//! declared to the run at its start (`declare`) with their columns and their supplier, so the executor holds no
//! language's fact by name: the provider's own facts (written in `bench.rs`' `ColFacts`), the host's relations, and the
//! provider's project relations (redo-general: its packaging knowledge, `provider_packaging.rs`).

use super::finish::{self, Supplier, Ty, project::ProjectAsk};

/// 6f probe: the provider's evaluation region of every node, by name.
pub(super) const REGION_REL: &str = "evaluation-region";
pub(super) const REGION_COLS: [(&str, Ty); 6] =
    [("id", Ty::Node), ("region", Ty::Node), ("region-kind", Ty::Sym), ("annotation", Ty::Node), ("timing", Ty::Sym), ("file", Ty::File)];

/// redo-binding probe: the provider's resolution of every read and deletion, one row each: the event in effect in the nearest
/// namespace of its lookup path that holds one (a deletion looks in its own namespace only), by name. `kind` is that event's
/// kind (`none`: no namespace holds an event of the name), `namespace` says whether the event was made in that namespace
/// itself (`own`) or routed into it by a `global` / `nonlocal` declaration elsewhere (`routed`).
/// redo-binding pass 3: the event in effect for each name in each namespace once the schedule has run (the namespace's state
/// after the module ran: what a postponed annotation sees, PEP 563), one row per (namespace, name) that holds an event.
pub(super) const FINAL_REL: &str = "namespace-final";
pub(super) const FINAL_COLS: [(&str, Ty); 6] =
    [("namespace", Ty::Node), ("name", Ty::Sym), ("event", Ty::Node), ("kind", Ty::Sym), ("ordinal", Ty::Num), ("file", Ty::File)];

/// redo-binding pass 4: every element of a file in a type expression (`type`), or the root of a value expression nested in
/// one (`value`); no row: a value expression (`type_expression.rs`).
pub(super) const TYPE_EXPRESSION_REL: &str = "type-expression";
pub(super) const TYPE_EXPRESSION_COLS: [(&str, Ty); 3] = [("id", Ty::Node), ("kind", Ty::Sym), ("file", Ty::File)];

/// redo-binding pass 4: every element inside an annotation, when Python evaluates it (`annotation_evaluation.rs`).
pub(super) const ANNOTATION_EVALUATION_REL: &str = "annotation-evaluation";
pub(super) const ANNOTATION_EVALUATION_COLS: [(&str, Ty); 5] =
    [("id", Ty::Node), ("root", Ty::Node), ("evaluation", Ty::Sym), ("mode", Ty::Sym), ("file", Ty::File)];

/// redo-binding pass 4: every element whose evaluation is postponed past its statement (`deferred`) (`deferred_visit.rs`).
pub(super) const DEFERRED_VISIT_REL: &str = "deferred-visit";
pub(super) const DEFERRED_VISIT_COLS: [(&str, Ty); 3] = [("id", Ty::Node), ("visit", Ty::Sym), ("file", Ty::File)];

/// redo-binding pass 4: every postponed read's event once the module has run (`deferred_resolution.rs`).
pub(super) const DEFERRED_RESOLUTION_REL: &str = "deferred-resolution";
pub(super) const DEFERRED_RESOLUTION_COLS: [(&str, Ty); 5] =
    [("reference", Ty::Node), ("occurrence", Ty::Node), ("kind", Ty::Sym), ("point", Ty::Num), ("file", Ty::File)];

/// redo-binding pass 4: the elements evaluation-region places in a deferred type phase, alone (`ColumnFile::type_regions`).
pub(super) const TYPE_REGION_REL: &str = "type-region";
pub(super) const TYPE_REGION_COLS: [(&str, Ty); 2] = [("id", Ty::Node), ("file", Ty::File)];

pub(super) const RESOLUTION_REL: &str = "read-resolution";
pub(super) const RESOLUTION_COLS: [(&str, Ty); 9] = [
    ("reference", Ty::Node),
    ("event", Ty::Node),
    ("kind", Ty::Sym),
    ("namespace", Ty::Sym),
    ("scope", Ty::Node),
    ("depth", Ty::Num),
    ("ordinal", Ty::Num),
    ("context", Ty::Sym),
    ("file", Ty::File),
];


/// Declares to `run` every relation this provider, the host and the executor's steps supply beyond the provider's relations
/// catalog.
pub(super) fn declare(run: &mut finish::Run) {
    let facts: [(&str, &[(&str, Ty)]); 8] = [
        (REGION_REL, &REGION_COLS),
        (RESOLUTION_REL, &RESOLUTION_COLS),
        (FINAL_REL, &FINAL_COLS),
        (TYPE_EXPRESSION_REL, &TYPE_EXPRESSION_COLS),
        (ANNOTATION_EVALUATION_REL, &ANNOTATION_EVALUATION_COLS),
        (DEFERRED_VISIT_REL, &DEFERRED_VISIT_COLS),
        (DEFERRED_RESOLUTION_REL, &DEFERRED_RESOLUTION_COLS),
        (TYPE_REGION_REL, &TYPE_REGION_COLS),
    ];
    for (name, cols) in facts {
        run.declare(name, cols, Supplier::Provider);
    }
    // the host's: the run's files, and the file system's facts of a file (their columns: the relations catalog's)
    // redo-binding c32: `filesystem.file` is the current file's row (a host relation of the file), not the run's files
    run.declare("filesystem.file", &[("path", Ty::File)], Supplier::Host);
    for name in ["filesystem.executable", "filesystem.read-failure"] {
        if let Some((_, names, tys)) = run.catalog(name) {
            let cols: Vec<(String, Ty)> = names.iter().cloned().zip(tys.iter().copied()).collect();
            let cols: Vec<(&str, Ty)> = cols.iter().map(|(n, t)| (n.as_str(), *t)).collect();
            run.declare(name, &cols, Supplier::Host);
        }
    }
    // redo-general: the provider's project relations (`provider_packaging.rs`; their columns: the relations catalog's), asked
    // for a project: module identity from the project's layout alone (once a run), the namespace membership of each file's
    // imports from that file as the provider reads it
    use super::provider_packaging::{MEMBERSHIP_REL, MODULE_IDENTITY_REL};
    for (name, ask) in [(MODULE_IDENTITY_REL, ProjectAsk::Run), (MEMBERSHIP_REL, ProjectAsk::EachFile)] {
        if let Some((_, names, tys)) = run.catalog(name) {
            let cols: Vec<(String, Ty)> = names.iter().cloned().zip(tys.iter().copied()).collect();
            let cols: Vec<(&str, Ty)> = cols.iter().map(|(n, t)| (n.as_str(), *t)).collect();
            run.declare(name, &cols, Supplier::Project(ask));
        }
    }
}

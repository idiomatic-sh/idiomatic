//! redo-fills pass 2 (THROWAWAY, bead ryo-fmx3.1): the facts this prototype provider states beyond its relations catalog,
//! declared to the run at its start (`declare`) with their columns and their supplier, so the executor holds no
//! language's fact by name: the provider's own facts (written in `bench.rs`' `ColFacts`), the host's relations, and the
//! provider's project relations (redo-general: its packaging knowledge, `provider_packaging.rs`).

use super::finish::{self, Supplier, Ty, project::ProjectAsk};

/// Syntactic stores and unbindings for a single name in one namespace and evaluation phase.
pub(super) const BINDING_SITES_REL: &str = "binding-sites";

/// Source assignment targets, their whole or statically unpacked RHS and assignment form.
pub(super) const ASSIGNMENT_SOURCE_REL: &str = "assignment-source";
/// First declarations and first mutations of a name in a routed namespace.
pub(super) const NAMESPACE_SITES_REL: &str = "namespace-sites";

/// Reads before the first scheduled store or annotation of their name in the same namespace and phase.
pub(super) const PRE_DECLARATION_READ_REL: &str = "pre-declaration-read";
const PRE_DECLARATION_READ_COLS: [(&str, Ty); 5] = [("reference", Ty::Node), ("scope", Ty::Node), ("phase", Ty::Node), ("ordinal", Ty::Num), ("file", Ty::File)];


/// Dotted name requested by a relative import when its containing package is known.
pub(super) const RELATIVE_IMPORT_REL: &str = "relative-import-target";
const RELATIVE_IMPORT_COLS: [(&str, Ty); 3] = [("occurrence", Ty::Node), ("path", Ty::Sym), ("file", Ty::File)];

/// 6f probe: the provider's evaluation region of every node, by name.
pub(super) const REGION_REL: &str = "evaluation-region";
pub(super) const REGION_COLS: [(&str, Ty); 6] =
    [("id", Ty::Node), ("region", Ty::Node), ("region-kind", Ty::Sym), ("annotation", Ty::Node), ("timing", Ty::Sym), ("file", Ty::File)];

/// Last scheduled namespace change per name, including declarations that redirect subsequent writes.
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

/// The module lookup for each postponed read at the end of the static schedule.
pub(super) const DEFERRED_RESOLUTION_REL: &str = "deferred-resolution";
pub(super) const DEFERRED_RESOLUTION_COLS: [(&str, Ty); 5] =
    [("reference", Ty::Node), ("occurrence", Ty::Node), ("kind", Ty::Sym), ("point", Ty::Num), ("file", Ty::File)];

/// redo-binding pass 4: the elements evaluation-region places in a deferred type phase, alone (`ColumnFile::type_regions`).
pub(super) const TYPE_REGION_REL: &str = "type-region";
pub(super) const TYPE_REGION_COLS: [(&str, Ty); 2] = [("id", Ty::Node), ("file", Ty::File)];

/// Nearest event on the provider's scheduled lookup path, including a marker for an empty path.
pub(super) const PREDECESSOR_REL: &str = "scheduled-read-predecessor";
/// One row per possible flow outcome at a read, with the scheduled predecessor recorded separately.
pub(super) const RESOLUTION_REL: &str = "read-resolution";



/// Declares to `run` every relation this provider, the host and the executor's steps supply beyond the provider's relations
/// catalog.
pub(super) fn declare(run: &mut finish::Run) {
    let columns = |fields: &[(&'static str, ryo::languages::CellKind)]| -> Vec<_> {
        fields.iter().map(|&(n, k)| (n, finish::ty_of(k))).collect()
    };
    let sites = columns(super::pair_provider::SITES);
    let mut resolutions = columns(super::pair_provider::predecessor::COLUMNS);
    let assignments = columns(super::pair_provider::sites::ASSIGNMENT);
    let namespaces = columns(super::pair_provider::sites::NAMESPACE);
    let facts: &[(&str, &[(&str, Ty)])] = &[
        (ASSIGNMENT_SOURCE_REL, &assignments),
        (PRE_DECLARATION_READ_REL, &PRE_DECLARATION_READ_COLS),
        (NAMESPACE_SITES_REL, &namespaces),
        (REGION_REL, &REGION_COLS),
        (BINDING_SITES_REL, &sites),
        (RELATIVE_IMPORT_REL, &RELATIVE_IMPORT_COLS),
        (PREDECESSOR_REL, &resolutions),
        (FINAL_REL, &FINAL_COLS),
        (TYPE_EXPRESSION_REL, &TYPE_EXPRESSION_COLS),
        (ANNOTATION_EVALUATION_REL, &ANNOTATION_EVALUATION_COLS),
        (DEFERRED_VISIT_REL, &DEFERRED_VISIT_COLS),
        (DEFERRED_RESOLUTION_REL, &DEFERRED_RESOLUTION_COLS),
        (TYPE_REGION_REL, &TYPE_REGION_COLS),
    ];
    for &(name, cols) in facts {
        run.declare(name, cols, Supplier::Provider);
    }
    resolutions.extend([("binding", Ty::Node), ("outcome", Ty::Sym), ("basis", Ty::Sym)]);
    run.declare(RESOLUTION_REL, &resolutions, Supplier::Provider);
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

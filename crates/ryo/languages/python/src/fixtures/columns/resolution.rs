//! Language resolution supplied by the file's flow model.
use super::ColumnFile;
use crate::domain::model::declarations::service::redirected;
use crate::domain::model::flow::types::{FlowEventKind, FlowRead, FlowState};
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::lexical::service::{entry, scope_at, scope_form};
use crate::domain::model::lexical::types::{NamespaceKey, ScopeForm};
use crate::domain::model::reaching::types::FlowReadState;
use crate::domain::model::types::{BindingAction, BindingPhaseKind, BindingStep};
use crate::domain::model::view::types::{ModelPart, ModelView, SemanticModel};
use crate::domain::node::service::table_id;
use ryo::languages::Position;
use std::collections::{HashMap, HashSet};

/// Find the slot of a read with no reaching binding. A class encloses its own body only.
fn declared_slot(model: &SemanticModel, read: &FlowRead) -> NamespaceKey {
    let lexical = &model.lexical;
    let mut scope = redirected(lexical, read.scope, read.name);
    loop {
        let key = NamespaceKey {
            scope,
            name: read.name,
        };
        if scope == MODULE
            || ((scope == read.scope
                || scope_form(entry(&lexical.class, scope)) != Some(ScopeForm::Class))
                && (model.definitions.index.contains_key(&key)
                    || model.flow.reach.mutations.contains_key(&key)))
        {
            return key;
        }
        scope = redirected(lexical, scope_at(lexical, scope), read.name);
    }
}

/// Mutations of the same name in one namespace and execution phase, with the statement containing the binding.
#[derive(Clone, Copy, Debug)]
pub struct BindingSite {
    /// Source occurrence that binds the name.
    pub binding: u32,
    /// Store sites in this phase.
    pub stores: usize,
    /// Deletions and exception cleanups in this phase.
    pub removals: usize,
    /// Subsequent mutations in the phase, in schedule order.
    pub later: usize,
    /// Last binding before the next deletion/cleanup in this namespace and phase.
    pub series_end: u32,
    /// The next store when no removal intervenes.
    pub next: Option<u32>,
    /// Innermost statement containing the occurrence.
    pub statement: Option<u32>,
}

#[derive(Default)]
struct SiteSequence {
    stores: usize,
    removals: usize,
    later: usize,
    end: Option<Position>,
    next: Option<Position>,
}

impl SiteSequence {
    fn count(&mut self, step: &BindingStep) {
        if step.action == BindingAction::Bind {
            self.stores += 1;
        } else {
            self.removals += 1;
        }
    }

    /// In reverse evaluation order each removal ends the uninterrupted sequence of stores.
    fn previous(&mut self, step: &BindingStep, write: &mut impl FnMut(Position, &Self, Position)) {
        if step.action == BindingAction::Bind {
            let end = *self.end.get_or_insert(step.occurrence);
            write(step.occurrence, self, end);
            self.next = Some(step.occurrence);
        } else {
            self.end = None;
            self.next = None;
        }
        self.later += 1;
    }
}

impl ColumnFile<'_, '_> {
    /// A read has each reaching binding or absence of the flow. An open invocation or inspection also sees
    /// the source writes to its captured slot, and an unknown outcome for dynamically supplied or removed values.
    pub fn visit_read_outcomes(
        &mut self,
        mut visit: impl FnMut(u32, Option<u32>, &'static str, &'static str),
    ) -> bool {
        let model = self.file.semantics(ModelPart::Flow);
        let flow = &model.model.flow;
        if flow.state == FlowState::Incomplete {
            return false;
        }
        let segments = model.tree.segments;
        let target = model.model.target;
        let writes: HashSet<_> = flow
            .events
            .iter()
            .filter(|event| {
                event.scope != event.lexical
                    && matches!(
                        event.kind,
                        FlowEventKind::Bind | FlowEventKind::Delete | FlowEventKind::Cleanup
                    )
            })
            .map(|event| NamespaceKey {
                scope: event.scope,
                name: event.name,
            })
            .collect();
        let late_types: HashSet<_> = model
            .model
            .schedule
            .phases
            .iter()
            .filter(|phase| phase.kind == BindingPhaseKind::Type)
            .map(|phase| phase.root)
            .collect();
        // A module wildcard can replace a builtin or earlier module binding with an unknown export.
        let mut star_at_read = HashMap::new();
        let mut module_stores = HashMap::new();
        if flow
            .events
            .iter()
            .any(|e| e.kind == FlowEventKind::Wildcard && e.scope == MODULE)
        {
            let mut star = None;
            for e in flow.events.iter().filter(|e| e.phase == MODULE) {
                match e.kind {
                    FlowEventKind::Wildcard if e.scope == MODULE => star = Some(e.ordinal.0),
                    FlowEventKind::Bind if e.scope == MODULE => {
                        let _ = module_stores.insert(e.at, e.ordinal.0);
                    }
                    FlowEventKind::Read | FlowEventKind::Delete => {
                        if let Some(point) = star {
                            let _ = star_at_read.insert(e.at, point);
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut edges = flow.uses.iter().peekable();
        let mut bindings = Vec::new();
        let mut scopes = Vec::new();
        for read in &flow.reads {
            let id = table_id(segments, read.at).0;
            bindings.clear();
            scopes.clear();
            while let Some(edge) = edges.next_if(|edge| edge.at == read.at) {
                if !bindings.contains(&edge.binding) {
                    bindings.push(edge.binding);
                    visit(
                        id,
                        Some(table_id(segments, edge.binding).0),
                        "binding",
                        "flow",
                    );
                }
                if scopes.last() != Some(&edge.scope) {
                    scopes.push(edge.scope);
                }
            }
            let fallback = match read.state {
                FlowReadState::Unbound | FlowReadState::PossiblyUnbound => Some("unbound"),
                FlowReadState::Builtin(since) | FlowReadState::DeclarationOrBuiltin(since) => Some(
                    if (target.major.0, target.minor.0) >= (since.major.0, since.minor.0) {
                        "builtin"
                    } else {
                        "unbound"
                    },
                ),
                FlowReadState::Uncertain => Some("unknown"),
                FlowReadState::Unique | FlowReadState::Alternatives => None,
            };
            if let Some(outcome) = fallback {
                visit(id, None, outcome, "flow");
            }
            if scopes.is_empty() {
                scopes.push(declared_slot(model.model, read).scope);
            }
            let late_type = late_types.contains(&read.phase);
            let wildcard_lookup = scopes.contains(&MODULE) || fallback == Some("builtin");
            let mut open = late_type
                || (wildcard_lookup
                    && star_at_read.get(&read.at).is_some_and(|star| {
                        fallback.is_some()
                            || bindings.iter().any(|binding| {
                                module_stores.get(binding).is_some_and(|at| at < star)
                            })
                    }));
            for &scope in &scopes {
                let key = NamespaceKey {
                    scope,
                    name: read.name,
                };
                let other_invocation = read.phase != MODULE && scope != read.phase;
                let redirected_write = writes.contains(&key);
                if !(other_invocation || redirected_write || late_type) {
                    continue;
                }
                open = true;
                if let Some(index) = flow.reach.mutations.get(&key) {
                    for step in &flow.reach.mutation_lists[usize::from(*index)] {
                        // A handler or another invocation may update the cell. The enclosing invocation's
                        // ordinary stores already have reaching edges. They must not mask an early local.
                        if step.action != BindingAction::Bind
                            || (!other_invocation && !late_type && step.phase == read.phase)
                        {
                            continue;
                        }
                        if !bindings.contains(&step.occurrence) {
                            bindings.push(step.occurrence);
                            visit(
                                id,
                                Some(table_id(segments, step.occurrence).0),
                                "binding",
                                "namespace",
                            );
                        }
                    }
                }
            }
            if open && fallback != Some("unknown") {
                visit(id, None, "unknown", "open");
            }
        }
        edges.next().is_none()
    }

    /// Each reachable binding has its lowest syntax-node read for flow and for additional open-namespace outcomes.
    pub fn visit_binding_read_witnesses(
        &mut self,
        mut visit: impl FnMut(u32, Option<u32>, Option<u32>),
    ) -> bool {
        let mut reads = std::collections::BTreeMap::<u32, [Option<u32>; 2]>::new();
        let complete = self.visit_read_outcomes(|reference, binding, _, basis| {
            if let Some(binding) = binding {
                let slot =
                    &mut reads.entry(binding).or_default()[usize::from(basis == "namespace")];
                *slot = Some(slot.map_or(reference, |old| old.min(reference)));
            }
        });
        if complete {
            for (binding, [flow, namespace]) in reads {
                visit(binding, flow, namespace);
            }
        }
        complete
    }

    /// The last event in the static evaluation schedule is the largest ordinal of a binding event.
    pub fn binding_schedule_end(&mut self) -> Result<Option<u64>, &'static str> {
        let view = self.file.semantics(ModelPart::Flow);
        if view.model.flow.state == FlowState::Incomplete {
            return Err("incomplete binding schedule");
        }
        Ok(view
            .model
            .flow
            .events
            .iter()
            .map(|event| event.ordinal.0 as u64)
            .max())
    }

    /// Count the store and unbinding sites of a name in its routed namespace and evaluation phase.
    /// Later sites count mutations after this binding in schedule order, including the cleanup of the same handler target.
    pub fn visit_binding_sites(&mut self, mut visit: impl FnMut(BindingSite)) -> bool {
        let read = self.file.semantics(ModelPart::Flow);
        let flow = &read.model.flow;
        if flow.state == FlowState::Incomplete {
            return false;
        }
        let tree = read.tree;
        let mut write = |at, sequence: &SiteSequence, end| {
            use crate::domain::model::lexical::service::{node_at, parent_of};
            let mut parent = at;
            let statement = loop {
                if node_at(tree, parent).is_some_and(|node| node.is_statement()) {
                    break Some(table_id(tree.segments, parent).0);
                }
                let next = parent_of(tree, parent);
                if next == parent {
                    break None;
                }
                parent = next;
            };
            visit(BindingSite {
                binding: table_id(tree.segments, at).0,
                stores: sequence.stores,
                removals: sequence.removals,
                later: sequence.later,
                series_end: table_id(tree.segments, end).0,
                next: sequence.next.map(|at| table_id(tree.segments, at).0),
                statement,
            });
        };
        let mut phases = HashMap::new();
        for (key, &index) in &flow.reach.mutations {
            if key.name == crate::domain::model::names::constants::UNKNOWN_NAME {
                continue;
            }
            let list = &flow.reach.mutation_lists[usize::from(index)];
            let Some(last) = list.last() else { continue };
            if list.iter().all(|step| step.phase == last.phase) {
                let mut sequence = SiteSequence::default();
                for step in list {
                    sequence.count(step);
                }
                for step in list.iter().rev() {
                    sequence.previous(step, &mut write);
                }
            } else {
                phases.clear();
                for step in list {
                    phases
                        .entry(step.phase)
                        .or_insert_with(SiteSequence::default)
                        .count(step);
                }
                for step in list.iter().rev() {
                    phases
                        .get_mut(&step.phase)
                        .expect("counted phase")
                        .previous(step, &mut write);
                }
            }
        }
        true
    }
}

impl ColumnFile<'_, '_> {
    /// For a named namespace, report its first scheduled declaration or mutation, its first mutation in
    /// schedule order, and the earliest written mutation in source order. Reads do not change a namespace.
    pub fn visit_namespace_sites(
        &mut self,
        mut visit: impl FnMut(u32, &str, u64, Option<u64>, Option<u32>),
    ) -> bool {
        use crate::domain::model::names::service::text_of;
        let table = self.table;
        let read = self.file.semantics(ModelPart::Flow);
        if read.model.flow.state == FlowState::Incomplete {
            return false;
        }
        let mut sites = HashMap::new();
        for event in &read.model.flow.events {
            match event.kind {
                FlowEventKind::Read | FlowEventKind::Wildcard => continue,
                _ => {}
            }
            let key = NamespaceKey {
                scope: event.scope,
                name: event.name,
            };
            let ordinal = event.ordinal.0 as u64;
            let entry = sites.entry(key).or_insert((ordinal, None, None));
            entry.0 = entry.0.min(ordinal);
            if matches!(
                event.kind,
                FlowEventKind::Bind | FlowEventKind::Delete | FlowEventKind::Cleanup
            ) {
                entry.1 = Some(entry.1.map_or(ordinal, |prior: u64| prior.min(ordinal)));
                let node = table_id(read.tree.segments, event.at);
                let start = ryo::languages::node_table::span_of(table, node).start.0;
                entry.2 = Some(entry.2.map_or(start, |prior: u32| prior.min(start)));
            }
        }
        for (key, (first, mutation, written)) in sites {
            if let Some(name) = text_of(&read.model.names, key.name) {
                visit(
                    table_id(read.tree.segments, key.scope).0,
                    name.0,
                    first,
                    mutation,
                    written,
                );
            }
        }
        true
    }
}

impl ColumnFile<'_, '_> {
    /// Schemas for rows over the parsed file's syntax and binding schedule.
    pub const SITE_SCHEMAS: &'static [ryo::languages::FileRelation] = {
        use ryo::languages::{
            CellKind::{File, Node, Number, Symbol},
            FileRelation,
        };
        &[
            FileRelation {
                name: "assignment-source",
                columns: &[
                    ("target", Node),
                    ("value", Node),
                    ("operation", Symbol),
                    ("association", Symbol),
                    ("file", File),
                ],
            },
            FileRelation {
                name: "binding-read-summary",
                columns: &[
                    ("binding", Node),
                    ("flow-read", Node),
                    ("namespace-read", Node),
                    ("file", File),
                ],
            },
            FileRelation {
                name: "binding-sites",
                columns: &[
                    ("binding", Node),
                    ("stores", Number),
                    ("removals", Number),
                    ("later", Number),
                    ("statement", Node),
                    ("series-end", Node),
                    ("next", Node),
                    ("file", File),
                ],
            },
            FileRelation {
                name: "namespace-sites",
                columns: &[
                    ("scope", Node),
                    ("name", Symbol),
                    ("first", Number),
                    ("first-mutation", Number),
                    ("first-written", Number),
                    ("file", File),
                ],
            },
            FileRelation {
                name: "pre-declaration-read",
                columns: &[
                    ("reference", Node),
                    ("scope", Node),
                    ("phase", Node),
                    ("ordinal", Number),
                    ("file", File),
                ],
            },
            FileRelation {
                name: "type-region",
                columns: &[("id", Node), ("file", File)],
            },
            FileRelation {
                name: "namespace-final",
                columns: &[
                    ("namespace", Node),
                    ("name", Symbol),
                    ("event", Node),
                    ("kind", Symbol),
                    ("ordinal", Number),
                    ("file", File),
                ],
            },
        ]
    };

    /// Complete binding, assignment and type-region columns in the syntax dictionary.
    pub fn named_site(
        &mut self,
        schema: &ryo::languages::FileRelation,
        file: ryo::languages::FileId,
        src: &[u8],
        symbols: &mut ryo::languages::Symbols,
    ) -> Result<Vec<ryo::languages::FactColumn>, String> {
        use ryo::languages as l;
        if !Self::SITE_SCHEMAS.contains(schema) {
            return Err(format!("no site relation {}", schema.name));
        }
        if self.file.syntax.completeness.syntax != l::SyntaxCompleteness::Complete {
            return Err(format!("incomplete {}", schema.name));
        }
        let mut columns = vec![Vec::new(); schema.columns.len()];
        self.visit_site_relation(
            schema.name,
            u64::from(file.0),
            |text| u64::from(l::intern_derived(symbols, l::ByteSlice(src), l::NameText(text)).0),
            |row| {
                debug_assert_eq!(columns.len(), row.len());
                for (out, &cell) in columns.iter_mut().zip(row) {
                    out.push(cell);
                }
            },
        )?;
        Ok(qualify_cells(schema, file, columns))
    }

    /// Visit site rows using file-local node identities and the supplied symbol dictionary.
    pub fn visit_site_relation(
        &mut self,
        relation: &str,
        file: u64,
        mut word: impl FnMut(&str) -> u64,
        mut row: impl FnMut(&[u64]),
    ) -> Result<(), String> {
        const NULL: u64 = u64::MAX;
        let complete = match relation {
            "assignment-source" => {
                self.visit_assignment_sources(|at, value, operation, association| {
                    row(&[
                        at.into(),
                        value.into(),
                        word(operation),
                        word(association),
                        file,
                    ]);
                });
                true
            }
            "binding-read-summary" => {
                self.visit_binding_read_witnesses(|binding, flow, namespace| {
                    row(&[
                        binding.into(),
                        flow.map_or(NULL, u64::from),
                        namespace.map_or(NULL, u64::from),
                        file,
                    ]);
                })
            }
            "binding-sites" => self.visit_binding_sites(|site| {
                row(&[
                    site.binding.into(),
                    site.stores as u64,
                    site.removals as u64,
                    site.later as u64,
                    site.statement.map_or(NULL, u64::from),
                    site.series_end.into(),
                    site.next.map_or(NULL, u64::from),
                    file,
                ]);
            }),
            "namespace-sites" => {
                self.visit_namespace_sites(|scope, name, first, mutation, written| {
                    row(&[
                        scope.into(),
                        word(name),
                        first,
                        mutation.unwrap_or(NULL),
                        written.map_or(NULL, u64::from),
                        file,
                    ]);
                })
            }
            "pre-declaration-read" => {
                self.visit_pre_declaration_reads(|at, scope, phase, ordinal| {
                    row(&[at.into(), scope.into(), phase.into(), ordinal, file]);
                })
            }
            "type-region" => {
                let ids = self.type_regions().ok_or("type-region: incomplete model")?;
                for id in ids {
                    row(&[id.into(), file]);
                }
                true
            }
            "namespace-final" => {
                self.visit_namespace_final(|scope, name, occurrence, kind, ordinal| {
                    row(&[
                        scope.into(),
                        word(name),
                        occurrence.into(),
                        word(kind),
                        ordinal,
                        file,
                    ]);
                })
            }
            _ => return Err(format!("no site relation {relation}")),
        };
        if !complete {
            return Err(format!("{relation}: incomplete flow"));
        }
        Ok(())
    }

    fn visit_namespace_final(
        &mut self,
        mut row: impl FnMut(u32, &str, u32, &'static str, u64),
    ) -> bool {
        use crate::domain::binding_facts::service::event_word;
        use crate::domain::model::names::service::text_of;
        let view = self.file.semantics(ModelPart::Flow);
        if view.model.flow.state == FlowState::Incomplete {
            return false;
        }
        let segments = view.tree.segments;
        let mut latest = HashMap::new();
        let mut events: Vec<_> = view.model.flow.events.iter().collect();
        events.sort_by_key(|event| event.ordinal);
        for event in events {
            match event.kind {
                FlowEventKind::Read | FlowEventKind::Wildcard => continue,
                FlowEventKind::Global | FlowEventKind::Nonlocal if event.lexical != event.scope => {
                    let _ = latest.insert((event.lexical, event.name), event);
                }
                _ => {}
            }
            let _ = latest.insert((event.scope, event.name), event);
        }
        let mut ordered: Vec<_> = latest.into_iter().collect();
        ordered.sort_unstable_by_key(|((scope, name), _)| (table_id(segments, *scope).0, *name));
        for ((scope, name), event) in ordered {
            row(
                table_id(segments, scope).0,
                text_of(&view.model.names, name).map_or("", |text| text.0),
                table_id(segments, event.at).0,
                event_word(event.kind).0,
                event.ordinal.0 as u64,
            );
        }
        true
    }
}

#[path = "named_semantics.rs"]
mod named_semantics;

#[path = "named.rs"]
mod named;

pub(super) fn qualify_cells(
    schema: &ryo::languages::FileRelation,
    file: ryo::languages::FileId,
    columns: Vec<Vec<u64>>,
) -> Vec<ryo::languages::FactColumn> {
    columns
        .into_iter()
        .zip(schema.columns)
        .map(|(mut cells, (_, kind))| {
            if *kind == ryo::languages::CellKind::Node {
                for cell in &mut cells {
                    if *cell != u64::MAX {
                        *cell |= u64::from(file.0) << 32;
                    }
                }
            }
            ryo::languages::FactColumn(cells)
        })
        .collect()
}

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
    /// Before 3.14, without postponed annotations or generic declarations, only embedded strings defer evaluation in a runtime module.
    pub fn eager_module_segments(&self) -> Option<impl Iterator<Item = Position> + '_> {
        use crate::domain::nodes::types::Node;
        use crate::domain::relations::collections::relations::compiler_feature::item::feature_flag;
        use crate::domain::syntax::{constants::MODULE_FUTURE_ANNOTATIONS, service::kind_bits, types::Kind};
        use ryo::languages::{Flag, node_table};
        let target = self.file.target;
        let flag = node_table::property_of(self.table, Position(0), MODULE_FUTURE_ANNOTATIONS);
        if target.major.0 != 3 || target.minor.0 >= 14 || feature_flag(flag) != Some(Flag::Lowered) {
            return None;
        }
        for kind in [Node::TypeAlias, Node::TypeVar, Node::TypeVarTuple, Node::ParamSpec] {
            if !node_table::ids_of_kind(self.table, kind_bits(Kind::Node(kind))).is_empty() {
                return None;
            }
        }
        Some(self.file.syntax.tree.segments.iter().map(|segment| segment.root))
    }

    /// Each import's first scheduled reader, the keyword arguments containing it within its parsed expression,
    /// and the callable's proven origin when there is a single enclosing keyword.
    pub fn visit_keyword_origins(
        &mut self,
        samples: impl Iterator<Item = (Position, Position, bool)>,
        mut visit: impl FnMut(Position, Position, bool, usize, &str),
    ) {
        use crate::domain::lexical_facts::{service::qualify, types::{NameProvenance, QualifiedPath}};
        use crate::domain::model::lexical::service::node_at;
        use crate::domain::node::service::{id_of, place_of_id};
        use crate::domain::nodes::types::Node;
        use crate::domain::syntax::{service::kind_bits, types::Kind};
        use ruff_python_ast::AnyNodeRef;
        use ryo::languages::node_table;
        let keyword = kind_bits(Kind::Node(Node::Keyword));
        let tree = self.file.syntax.tree;
        let mut pending = Vec::new();
        for (binding, reference, own) in samples {
            let mut at = reference;
            let mut count = 0;
            let mut function = None;
            loop {
                let parent = node_table::parent_of(self.table, at);
                if parent == at || node_table::subtree_end_of(self.table, parent).0 <= at.0 { break; }
                if node_table::kind_of(self.table, at) == keyword {
                    count += 1;
                    if count == 1 {
                        let caller = node_table::parent_of(self.table, parent);
                        if let Some(place) = place_of_id(tree, caller)
                            && let Some(AnyNodeRef::ExprCall(call)) = node_at(tree, place)
                        {
                            function = id_of(&*call.func);
                        }
                    }
                }
                at = parent;
            }
            if count == 1 && let Some(function) = function {
                pending.push((binding, reference, own, function));
            } else {
                visit(binding, reference, own, count, "");
            }
        }
        if !pending.is_empty() {
            let read = self.file.semantics(ModelPart::Resolution);
            let mut path = QualifiedPath::default();
            for (binding, reference, own, function) in pending {
                let origin = match qualify(read.model, function, &mut path) {
                    NameProvenance::Builtin(_) | NameProvenance::Import => path.0.as_str(),
                    _ => "",
                };
                visit(binding, reference, own, 1, origin);
            }
        }
    }

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
                let slot = &mut reads.entry(binding).or_default()[usize::from(basis == "namespace")];
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
        Ok(view.model.flow.events.iter().map(|event| event.ordinal.0 as u64).max())
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
    pub fn visit_namespace_sites(&mut self, mut visit: impl FnMut(u32, &str, u64, Option<u64>, Option<u32>)) -> bool {
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
            let key = NamespaceKey { scope: event.scope, name: event.name };
            let ordinal = event.ordinal.0 as u64;
            let entry = sites.entry(key).or_insert((ordinal, None, None));
            entry.0 = entry.0.min(ordinal);
            if matches!(event.kind, FlowEventKind::Bind | FlowEventKind::Delete | FlowEventKind::Cleanup) {
                entry.1 = Some(entry.1.map_or(ordinal, |prior: u64| prior.min(ordinal)));
                let node = table_id(read.tree.segments, event.at);
                let start = ryo::languages::node_table::span_of(table, node).start.0;
                entry.2 = Some(entry.2.map_or(start, |prior: u32| prior.min(start)));
            }
        }
        for (key, (first, mutation, written)) in sites {
            if let Some(name) = text_of(&read.model.names, key.name) {
                visit(table_id(read.tree.segments, key.scope).0, name.0, first, mutation, written);
            }
        }
        true
    }
}

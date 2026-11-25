//! Language resolution supplied by the file's flow model.
use super::{ColumnFile, qualify_cells};
use crate::domain::model::flow::types::{FlowEventKind, FlowState};
use crate::domain::model::types::{BindingAction, BindingStep};
use crate::domain::model::view::types::{ModelPart, ModelView};
use crate::domain::node::service::table_id;
use ryo::languages::Position;
use std::collections::HashMap;

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
        let events = read.model.flow.events.iter().filter_map(|event| {
            if matches!(event.kind, FlowEventKind::Read | FlowEventKind::Wildcard) {
                return None;
            }
            let name = text_of(&read.model.names, event.name)?.0;
            let scope = table_id(read.tree.segments, event.scope);
            let write_at = matches!(
                event.kind,
                FlowEventKind::Bind | FlowEventKind::Delete | FlowEventKind::Cleanup
            )
            .then(|| {
                ryo::languages::node_table::span_of(table, table_id(read.tree.segments, event.at))
                    .start
                    .0
            });
            Some((scope, name, event.ordinal.0 as u64, write_at))
        });
        Self::visit_namespace_schedule(events, |scope, name, first, mutation, written| {
            visit(scope.0, name, first, mutation, written);
        });
        true
    }

    /// Project first declaration and mutation sites from the file's binding schedule.
    /// Each input is a namespace, its canonical name, a schedule ordinal, and the
    /// byte offset when the event changes the binding.
    pub fn visit_namespace_schedule<K: Eq + std::hash::Hash>(
        events: impl Iterator<Item = (Position, K, u64, Option<u32>)>,
        mut visit: impl FnMut(Position, K, u64, Option<u64>, Option<u32>),
    ) {
        let mut sites = HashMap::new();
        for (scope, name, ordinal, mutation) in events {
            let entry = sites.entry((scope, name)).or_insert((ordinal, None, None));
            entry.0 = entry.0.min(ordinal);
            if let Some(start) = mutation {
                entry.1 = Some(entry.1.map_or(ordinal, |prior: u64| prior.min(ordinal)));
                entry.2 = Some(entry.2.map_or(start, |prior: u32| prior.min(start)));
            }
        }
        for ((scope, name), (first, mutation, written)) in sites {
            visit(scope, name, first, mutation, written);
        }
    }

    /// Write namespace sites over the same public schedule as the binding-event rows.
    pub fn published_namespace_sites(
        file: ryo::languages::FileId,
        events: impl Iterator<Item = (Position, ryo::languages::Symbol, u64, Option<u32>)>,
    ) -> Vec<ryo::languages::FactColumn> {
        let mut columns = vec![Vec::new(); Self::NAMESPACE_SITES_SCHEMA.columns.len()];
        Self::visit_namespace_schedule(events, |scope, name, first, mutation, written| {
            let cells = [
                u64::from(scope.0),
                u64::from(name.0),
                first,
                mutation.unwrap_or(u64::MAX),
                written.map_or(u64::MAX, u64::from),
                u64::from(file.0),
            ];
            for (column, cell) in columns.iter_mut().zip(cells) {
                column.push(cell);
            }
        });
        qualify_cells(&Self::NAMESPACE_SITES_SCHEMA, file, columns)
    }
}

impl ColumnFile<'_, '_> {
    /// Scheduled mutations around each binding.
    pub const BINDING_SITES_SCHEMA: ryo::languages::FileRelation = {
        use ryo::languages::{
            CellKind::{File, Node, Number},
            FileRelation,
        };
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
        }
    };

    /// First events and mutations in a namespace.
    pub const NAMESPACE_SITES_SCHEMA: ryo::languages::FileRelation = {
        use ryo::languages::{
            CellKind::{File, Node, Number, Symbol},
            FileRelation,
        };
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
        }
    };

    /// Schemas for rows over the parsed file's syntax and binding schedule.
    pub const SITE_SCHEMAS: &'static [ryo::languages::FileRelation] = {
        use ryo::languages::{
            CellKind::{File, Node, Number},
            FileRelation,
        };
        &[
            Self::BINDING_SITES_SCHEMA,
            Self::NAMESPACE_SITES_SCHEMA,
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
        ]
    };

    /// Complete binding-site columns in the syntax dictionary.
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
            _ => return Err(format!("no site relation {relation}")),
        };
        if !complete {
            return Err(format!("{relation}: incomplete flow"));
        }
        Ok(())
    }
}

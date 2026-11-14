//! The file pass's service: the plan's root kinds, written as texts, mapped to the open file's own kind numbers, so each node
//! reading the tree starts from the tree nodes of its kinds, and a node with no root kind from every tree node.

use crate::application::file_dag::file_pass::types::{FileRoots, RootKindNodes, TreeStart};
use ryo_execution_domain::domain::fused_program::service::root_kinds;
use ryo_execution_domain::domain::fused_program::types::fused_program::value::FusedProgram;
use ryo_language::domain::elements::service::{kind_names, positions};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::of_kind;
use ryo_operators::domain::physical_plan::types::{Candidates, PlanNodeId};
use ryo_operators::domain::table_texts::service::symbol_text;
use ryo_operators::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{FileId, KindBits};

/// The roots of `program`, before any file.
pub(crate) fn file_roots(program: &FusedProgram) -> FileRoots {
    let kinds: Vec<RootKindNodes> = root_kinds(program)
        .map(|root| RootKindNodes {
            kind: root.kind.clone(),
            nodes: root.nodes.to_vec(),
        })
        .collect();
    let count = kinds
        .iter()
        .flat_map(|root| &root.nodes)
        .map(|node| usize::try_from(node.0).unwrap_or(usize::MAX).saturating_add(1))
        .max()
        .unwrap_or(0);
    let mut starts = vec![TreeStart::FromEveryNode; count];
    for node in kinds.iter().flat_map(|root| &root.nodes) {
        if let Some(start) = starts.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX)) {
            *start = TreeStart::FromKinds;
        }
    }
    FileRoots {
        kinds,
        starts,
        lists: vec![Vec::new(); count],
        all: Vec::new(),
    }
}

/// Makes the candidates of `file`, whose tree is `elements` and whose kind names read through `texts`.
pub(crate) fn fill_roots(roots: &mut FileRoots, elements: &Elements, texts: &Texts<'_>, file: FileId) {
    roots.all.clear();
    roots.all.extend(positions(elements).map(|at| node_id(file, at)));
    for list in &mut roots.lists {
        list.clear();
    }
    let names = kind_names(elements);
    for root in &roots.kinds {
        let numbered = names
            .iter()
            .position(|name| symbol_text(texts, *name).is_some_and(|found| found.0 == &*root.kind.0));
        // A kind the file does not name, or a number past the kinds a file can number, has no tree node in it.
        let Some(Ok(kind)) = numbered.map(u16::try_from) else {
            continue;
        };
        for node in &root.nodes {
            if let Some(list) = roots.lists.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX)) {
                list.extend(of_kind(elements, KindBits(kind)).iter().map(|at| node_id(file, *at)));
            }
        }
    }
    // A node of two kinds holds both kinds' tree nodes: within one file, tree order is node order.
    for list in &mut roots.lists {
        list.sort_unstable();
        list.dedup();
    }
}

/// The tree nodes `node` starts from in the open file, in tree order.
pub(crate) fn roots_of(roots: &FileRoots, node: PlanNodeId) -> Candidates<'_> {
    let at = usize::try_from(node.0).unwrap_or(usize::MAX);
    match (roots.starts.get(at), roots.lists.get(at)) {
        (Some(TreeStart::FromKinds), Some(list)) => Candidates(list),
        _ => Candidates(&roots.all),
    }
}

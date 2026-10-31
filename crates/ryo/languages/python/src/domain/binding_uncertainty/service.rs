//! Bound uncertainty to relevant namespace mutations, not all reads or whole-file capability absence.
use crate::domain::{
    binding_results::types::{Candidate, ReadState},
    definitions::service::definition,
    fields::service::field_of,
    labels::types::Field,
    model::types::{
        BindingAction, BindingAnalysisIndex, BindingKey, BindingMutation, Declarations,
    },
    nodes::types::Node,
    scopes::service::{binding_scope, enclosing},
    syntax::{
        service::{kind, name_of, parent},
        types::Kind,
    },
};
use ryo::languages::{Position, Symbol, Tree};

/// Index real mutation identities once, retaining the flat buffer between sources.
pub(crate) fn prepare(tree: &Tree<'_>, model: &Declarations, index: &mut BindingAnalysisIndex) {
    index.mutations.clear();
    for step in &index.schedule.steps {
        if !matches!(
            step.action,
            BindingAction::Bind | BindingAction::Delete | BindingAction::Cleanup
        ) {
            continue;
        }
        let declaration = definition(tree, model, step.occurrence);
        let Some(name) = declaration
            .map(|value| value.name)
            .or_else(|| name_of(tree, step.occurrence))
        else {
            continue;
        };
        let scope = declaration.map_or_else(
            || binding_scope(tree, model, step.occurrence, name),
            |value| value.owner,
        );
        index.mutations.push(BindingMutation {
            key: BindingKey { scope, name },
            occurrence: step.occurrence,
            phase: step.phase,
            action: step.action,
        });
    }
    index
        .mutations
        .sort_unstable_by_key(|mutation| (mutation.key, mutation.phase, mutation.occurrence));
}

/// Ancestor membership stops at the real source root; no textual containment substitutes for AST ownership.
fn within(tree: &Tree<'_>, mut at: Position, region: Position) -> bool {
    loop {
        if at == region {
            return true;
        }
        if at == Position(0) {
            return false;
        }
        let above = parent(tree, at);
        if above == at {
            return false;
        }
        at = above;
    }
}

/// A for iterable is evaluated once; comprehension first-iterable scope follows the same rule.
fn repeats_reference(tree: &Tree<'_>, reference: Position, region: Position) -> bool {
    match kind(tree, region) {
        Kind::Node(Node::While) => true,
        Kind::Node(Node::For) => {
            let mut at = reference;
            while parent(tree, at) != region {
                let above = parent(tree, at);
                if above == at || at == Position(0) {
                    return false;
                }
                at = above;
            }
            field_of(tree, at) != Field::Iter
        }
        Kind::Node(Node::ListComp | Node::SetComp | Node::DictComp | Node::GeneratorExp) => {
            within(tree, enclosing(tree, reference), region)
        }
        _ => false,
    }
}

/// Whether a mutation belongs to the protected try body, excluding else/finally and sibling handlers.
fn protected_body(tree: &Tree<'_>, mut at: Position, handler: Position) -> bool {
    let statement = parent(tree, handler);
    loop {
        let above = parent(tree, at);
        if above == statement {
            return field_of(tree, at) == Field::Body;
        }
        if above == at || at == Position(0) {
            return false;
        }
        at = above;
    }
}

/// Preserve candidates from loop iterations or exceptional prefixes without pretending that
/// a source-order visit proves runtime initialization. Queries still have exact events and ordinals
/// to express checker-specific pre-store policy. This is not a general definite-assignment solver.
pub(crate) fn refine(
    tree: &Tree<'_>,
    model: &Declarations,
    reference: Position,
    phase: Position,
    name: Symbol,
    state: ReadState,
    mutations: &[BindingMutation],
    candidates: &mut Vec<Candidate>,
) -> ReadState {
    let key = BindingKey {
        scope: binding_scope(tree, model, reference, name),
        name,
    };
    let start = mutations.partition_point(|mutation| mutation.key < key);
    let end = mutations.partition_point(|mutation| mutation.key <= key);
    let Some(relevant) = mutations.get(start..end) else {
        return state;
    };
    let mut at = reference;
    let mut uncertain = false;
    loop {
        let repeated = repeats_reference(tree, reference, at);
        let handler = kind(tree, at) == Kind::Node(Node::ExceptHandler);
        if (repeated || handler)
            && !(state == ReadState::UniqueDeclaration
                && candidates
                    .iter()
                    .all(|candidate| within(tree, candidate.binding, at)))
        {
            for mutation in relevant {
                if mutation.phase != phase
                    || !(if handler {
                        protected_body(tree, mutation.occurrence, at)
                    } else {
                        within(tree, mutation.occurrence, at)
                    })
                {
                    continue;
                }
                uncertain = true;
                if mutation.action == BindingAction::Bind
                    && !candidates
                        .iter()
                        .any(|candidate| candidate.binding == mutation.occurrence)
                {
                    candidates.push(Candidate {
                        binding: mutation.occurrence,
                        scope: mutation.key.scope,
                    });
                }
            }
        }
        if at == phase || at == Position(0) {
            break;
        }
        let above = parent(tree, at);
        if above == at {
            break;
        }
        at = above;
    }
    if uncertain {
        ReadState::Uncertain
    } else {
        state
    }
}

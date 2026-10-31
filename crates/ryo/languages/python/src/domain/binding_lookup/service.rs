//! Resolve static candidates without changing lexical resolves or asserting runtime initialization.
use crate::domain::{
    binding_results::types::{Candidate, ReadState},
    binding_states::service::{candidates, lookup},
    model::types::{
        BindingAbsence, BindingEnvironments, BindingFrameId, BindingKey, BindingKnowledge,
        Declarations,
    },
    nodes::types::Node,
    qualification::service::builtin,
    scopes::service::{enclosing, redirected},
    syntax::{service::kind, types::Kind},
};
use ryo::languages::{Position, Symbol, Tree};

/// Distinguish real alternatives from possible absence, with no shared fake binding identity.
fn classified(
    candidates: &[Candidate],
    absence: BindingAbsence,
    knowledge: BindingKnowledge,
) -> ReadState {
    if knowledge == BindingKnowledge::Uncertain {
        return ReadState::Uncertain;
    }
    match (candidates.len(), absence) {
        (0, _) => ReadState::Unbound,
        (_, BindingAbsence::Possible) => ReadState::PossiblyUnbound,
        (1, BindingAbsence::Excluded) => ReadState::UniqueDeclaration,
        (_, BindingAbsence::Excluded) => ReadState::AlternativeDeclarations,
    }
}

/// Associate a reference with real declarations or a builtin. A function read before its first
/// local event may associate an outer declaration for static shadow analysis; this does not
/// assert that Python would load that outer value at runtime. Once a local alternative exists,
/// an absent alternative stays absent rather than falling through that lexical local namespace.
pub(crate) fn resolve(
    tree: &Tree<'_>,
    model: &Declarations,
    state: &BindingEnvironments,
    frame: BindingFrameId,
    reference: Position,
    name: Symbol,
    output: &mut Vec<Candidate>,
) -> ReadState {
    output.clear();
    let initial = enclosing(tree, reference);
    let mut scope = redirected(tree, model, initial, name);
    let mut knowledge = BindingKnowledge::Known;
    loop {
        // Class namespaces are visible to their own body, not closure lexical lookup.
        let visible = scope == initial || kind(tree, scope) != Kind::Node(Node::ClassDef);
        if visible && let Some(value) = lookup(state, frame, BindingKey { scope, name }) {
            output.extend(candidates(state, value).iter().map(|binding| Candidate {
                binding: *binding,
                scope,
            }));
            if value.knowledge == BindingKnowledge::Uncertain {
                knowledge = BindingKnowledge::Uncertain;
            }
            if value.absence == BindingAbsence::Excluded {
                return classified(output, BindingAbsence::Excluded, knowledge);
            }
            if matches!(
                kind(tree, scope),
                Kind::Node(
                    Node::FunctionDef
                        | Node::Lambda
                        | Node::GeneratorExp
                        | Node::ListComp
                        | Node::SetComp
                        | Node::DictComp
                )
            ) {
                return classified(output, BindingAbsence::Possible, knowledge);
            }
        }
        if scope == Position(0) {
            break;
        }
        scope = redirected(tree, model, enclosing(tree, scope), name);
    }
    if knowledge == BindingKnowledge::Uncertain {
        return ReadState::Uncertain;
    }
    if let Some(version) = builtin(tree, reference) {
        if output.is_empty() {
            ReadState::Builtin(version)
        } else {
            ReadState::DeclarationOrBuiltin(version)
        }
    } else {
        classified(output, BindingAbsence::Possible, knowledge)
    }
}

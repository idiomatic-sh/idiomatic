//! Resolve static candidates without changing lexical resolves or asserting runtime initialization.
use crate::domain::{
    binding_results::types::{Candidate, ReadState},
    binding_states::service::{candidates, has_wildcard, lookup},
    fields::service::field_of,
    labels::types::Field,
    model::types::{
        BindingAbsence, BindingEnvironments, BindingFrameId, BindingKey, BindingKnowledge,
        Declarations,
    },
    nodes::types::Node,
    qualification::service::builtin,
    scopes::service::{enclosing, redirected},
    syntax::{
        service::{kind, nodes, parent},
        types::Kind,
    },
};
use ryo::languages::{Position, Symbol, Tree};

/// Annotation declarations own a lexical name but do not supply an initialized value.
fn annotation_only(tree: &Tree<'_>, at: Position) -> bool {
    let statement = parent(tree, at);
    kind(tree, statement) == Kind::Node(Node::AnnAssign)
        && !nodes(tree, statement).any(|child| field_of(tree, child) == Field::Value)
}

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
    let mut wildcard = BindingKnowledge::Known;
    loop {
        // Class namespaces are visible to their own body, not closure lexical lookup.
        let visible = scope == initial || kind(tree, scope) != Kind::Node(Node::ClassDef);
        if visible && has_wildcard(state, frame, scope) == BindingKnowledge::Uncertain {
            wildcard = BindingKnowledge::Uncertain;
        }
        if visible && let Some(value) = lookup(state, frame, BindingKey { scope, name }) {
            output.extend(
                candidates(state, value)
                    .iter()
                    .filter(|binding| !annotation_only(tree, **binding))
                    .map(|binding| Candidate {
                        binding: *binding,
                        scope,
                    }),
            );
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
            ReadState::Builtin {
                since_major: version.major,
                since_minor: version.minor,
            }
        } else {
            ReadState::DeclarationOrBuiltin {
                since_major: version.major,
                since_minor: version.minor,
            }
        }
    } else {
        if wildcard == BindingKnowledge::Uncertain {
            knowledge = BindingKnowledge::Uncertain;
        }
        classified(output, BindingAbsence::Possible, knowledge)
    }
}

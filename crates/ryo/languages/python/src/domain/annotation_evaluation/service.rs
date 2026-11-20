//! Annotation evaluation timing (Spike ryo-fmx3), from the language reference: function definitions (§8.7: parameter and return
//! annotations), annotated assignment statements (§7.2.2: module and class annotations are evaluated and stored, function-local ones
//! are not; non-simple targets), the annotations future (PEP 563) and native lazy annotations from Python 3.14 (PEP 649, PEP 749).
//! Independent of `.py`/`.pyi` file names: a stub typing policy is a consumer's. No linter policy here.
use super::types::{AnnotationEvaluationScratch, AnnotationMember, Evaluation, Mode};
use crate::domain::{
    binding_schedule::constants::LAZY_ANNOTATIONS_FROM,
    fields::service::field_of,
    labels::types::Field,
    model::types::Declarations,
    nodes::types::Node,
    syntax::{
        constants::{
            MODULE_FUTURE_ANNOTATIONS, MODULE_FUTURE_ANNOTATIONS_DISABLED,
            MODULE_FUTURE_ANNOTATIONS_ENABLED,
        },
        service::{is_root, kind, last, nodes, parent},
        types::{Kind, Rootness},
    },
};
use ryo::languages::{self, Position, Tree};

/// The annotations future of the file, when the frontend recorded it: `Some(true)` enabled, `Some(false)` disabled.
fn future(tree: &Tree<'_>) -> Option<bool> {
    match languages::property_at(tree.elements, Position(0), MODULE_FUTURE_ANNOTATIONS).map(|text| text.0) {
        Some(MODULE_FUTURE_ANNOTATIONS_ENABLED) => Some(true),
        Some(MODULE_FUTURE_ANNOTATIONS_DISABLED) => Some(false),
        _ => None,
    }
}

/// Where an annotation stands, which decides its timing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    /// A parameter or return annotation of a function definition.
    Signature,
    /// A module or class variable annotation whose target is an unparenthesized name.
    SimpleVariable,
    /// A module or class variable annotation with any other target.
    ComplexVariable,
    /// A variable annotation inside a function or lambda.
    LocalVariable,
}

/// The timing of an annotation at `place` in a file with the given future and native laziness (A3).
const fn timing(place: Place, enabled: bool, native: bool) -> Option<(Evaluation, Mode)> {
    let lazy = enabled || native;
    let mode = if lazy { Mode::Lazy } else { Mode::Eager };
    match place {
        Place::Signature | Place::SimpleVariable => Some(if enabled {
            (Evaluation::Postponed, mode)
        } else if native {
            (Evaluation::Deferred, mode)
        } else {
            (Evaluation::Eager, mode)
        }),
        Place::ComplexVariable => Some(if lazy {
            (Evaluation::Unevaluated, mode)
        } else {
            (Evaluation::Eager, mode)
        }),
        Place::LocalVariable => Some((Evaluation::Unevaluated, mode)),
    }
}

/// The file's rows; `None` when the annotations future is unknown.
pub(crate) fn evaluated<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
) -> Option<std::cell::Ref<'a, AnnotationEvaluationScratch>> {
    let enabled = future(tree)?;
    let native = model.language_target >= LAZY_ANNOTATIONS_FROM;
    {
        let mut out = model.annotation_evaluation.borrow_mut();
        out.members.clear();
        for at in languages::positions(tree.elements) {
            let (place, field) = match kind(tree, at) {
                Kind::Node(Node::Arg) if within_function(tree, at) => (Place::Signature, Field::Annotation),
                Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef) => (Place::Signature, Field::Returns),
                Kind::Node(Node::AnnAssign) => (variable_place(tree, at), Field::Annotation),
                _ => continue,
            };
            let Some(root) = nodes(tree, at).find(|each| field_of(tree, *each) == field) else {
                continue;
            };
            let Some((evaluation, mode)) = timing(place, enabled, native) else {
                continue;
            };
            members(tree, root, evaluation, mode, &mut out.members);
        }
    }
    Some(model.annotation_evaluation.borrow())
}

/// Whether a function definition holds the element at `at`.
fn within_function(tree: &Tree<'_>, mut at: Position) -> bool {
    while is_root(tree, at) == Rootness::Inner {
        at = parent(tree, at);
        if matches!(kind(tree, at), Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef)) {
            return true;
        }
    }
    false
}

/// The place of the annotated assignment at `at`: local when its nearest namespace is a function or lambda; otherwise simple when
/// its target is a name that starts where the statement starts (no parentheses).
fn variable_place(tree: &Tree<'_>, at: Position) -> Place {
    let mut above = at;
    while is_root(tree, above) == Rootness::Inner {
        above = parent(tree, above);
        match kind(tree, above) {
            Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef | Node::Lambda) => return Place::LocalVariable,
            Kind::Node(Node::ClassDef | Node::Module) => break,
            _ => {}
        }
    }
    let simple = nodes(tree, at).find(|each| field_of(tree, *each) == Field::Target).is_some_and(|target| {
        kind(tree, target) == Kind::Node(Node::Name)
            && languages::start_at(tree.elements, target) == languages::start_at(tree.elements, at)
    });
    if simple { Place::SimpleVariable } else { Place::ComplexVariable }
}

/// The elements of the annotation `root` (A2): the root and everything below it, except lambda bodies, a generator expression's
/// element and every part of its clauses but the first clause's iterable, which have their own evaluation (§6.2.8, §6.14).
fn members(tree: &Tree<'_>, root: Position, evaluation: Evaluation, mode: Mode, out: &mut Vec<AnnotationMember>) {
    let end = last(tree, root).0;
    let mut at = root.0;
    while at <= end {
        let here = Position(at);
        if here != root && separate(tree, here) {
            at = last(tree, here).0 + 1;
            continue;
        }
        out.push(AnnotationMember { id: here, root, evaluation, mode });
        at += 1;
    }
}

/// Whether the element at `at` starts a part evaluated on its own: a lambda body, a generator expression's element, or a child of a
/// generator expression's clause other than the first clause's iterable.
fn separate(tree: &Tree<'_>, at: Position) -> bool {
    let above = parent(tree, at);
    match (kind(tree, above), field_of(tree, at)) {
        (Kind::Node(Node::Lambda), Field::Body) | (Kind::Node(Node::GeneratorExp), Field::Elt) => true,
        (Kind::Node(Node::Comprehension), field) => {
            let generator = parent(tree, above);
            if kind(tree, generator) != Kind::Node(Node::GeneratorExp) {
                return false;
            }
            let first = nodes(tree, generator).find(|each| kind(tree, *each) == Kind::Node(Node::Comprehension));
            !(first == Some(above) && field == Field::Iter)
        }
        _ => false,
    }
}

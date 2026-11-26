//! When Python evaluates each element of an annotation (a parameter or return annotation, an annotated assignment's annotation):
//! `eager` (with its definition or statement, through 3.13 without the annotations future), `postponed` (stored as a string under
//! the future), `deferred` (stored lazily, 3.14 on), `unevaluated` (a function-local variable annotation, or a non-simple
//! target's under the lazy modes); the mode is `lazy` or `eager`, the root the annotation. An element inside a lambda body, a
//! generator's element or later clauses, or a string annotation's embedded expression has its own evaluation and no row.
//! The language reference's rules (§7.2.2 annotated assignment statements, PEP 563, PEP 649 / 749), written from the node table.

use super::constants::{CLASSES, MODULE};
use super::types::{Row, Target};
use crate::domain::fields::service::field_of;
use crate::domain::labels::service::{field_name, fields};
use crate::domain::syntax::service::{kind_name, kinds};
use ryo::languages::{self, Elements, NodeTable, Position, Tree, node_table};
use std::cell::RefCell;
use std::sync::OnceLock;

/// The kind names, by kind number.
fn kind_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| kinds().map(|kind| kind_name(kind).0).collect())
}

/// The field names, by field number.
fn field_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| fields().map(|field| field_name(field).0).collect())
}

/// What the rows are read from: the shape of a file's tree, by place. The walk's node table and the old route's elements both give it.
pub(crate) trait Shape {
    /// How many elements the tree has.
    fn count(&self) -> u32;
    /// The name of the kind of the element at `p`.
    fn kind(&self, p: u32) -> &'static str;
    /// The name of the field the element at `p` fills.
    fn field(&self, p: u32) -> &'static str;
    /// The parent of the element at `p`; the root is its own.
    fn parent(&self, p: u32) -> u32;
    /// The children of the element at `p`, in order.
    fn children(&self, p: u32) -> impl Iterator<Item = u32>;
    /// Where the element at `p` starts.
    fn start(&self, p: u32) -> u32;
    /// Whether any element is an annotation or a return annotation.
    fn has_annotation(&self) -> bool;
}

impl Shape for &NodeTable {
    fn count(&self) -> u32 {
        u32::try_from(node_table::element_count(self).0).unwrap_or(u32::MAX)
    }
    fn kind(&self, p: u32) -> &'static str {
        kind_names().get(usize::from(node_table::kind_of(self, Position(p)).0)).copied().unwrap_or("")
    }
    fn field(&self, p: u32) -> &'static str {
        field_names().get(usize::from(node_table::field_of(self, Position(p)).0)).copied().unwrap_or("")
    }
    fn parent(&self, p: u32) -> u32 {
        node_table::parent_of(self, Position(p)).0
    }
    fn children(&self, p: u32) -> impl Iterator<Item = u32> {
        node_table::children_of(self, Position(p)).iter().map(|c| c.0)
    }
    fn start(&self, p: u32) -> u32 {
        node_table::span_of(self, Position(p)).start.0
    }
    fn has_annotation(&self) -> bool {
        field_names().iter().enumerate().any(|(code, name)| {
            matches!(*name, "annotation" | "returns")
                && u16::try_from(code)
                    .is_ok_and(|code| !node_table::ids_of_field(self, ryo::languages::FieldCode(code)).is_empty())
        })
    }
}

/// The old route's elements, read as the tree of a file.
pub(crate) struct Old<'t, 'e>(pub(crate) &'t Tree<'e>);

impl Shape for Old<'_, '_> {
    fn count(&self) -> u32 {
        languages::end_of(self.0.elements).0
    }
    fn kind(&self, p: u32) -> &'static str {
        kind_names().get(usize::from(languages::kind_at(self.0.elements, Position(p)).0)).copied().unwrap_or("")
    }
    fn field(&self, p: u32) -> &'static str {
        field_name(field_of(self.0, Position(p))).0
    }
    fn parent(&self, p: u32) -> u32 {
        languages::parent_at(self.0.elements, Position(p)).0
    }
    fn children(&self, p: u32) -> impl Iterator<Item = u32> {
        languages::children(self.0.elements, Position(p)).map(|c| c.0)
    }
    fn start(&self, p: u32) -> u32 {
        languages::start_at(self.0.elements, Position(p)).0
    }
    fn has_annotation(&self) -> bool {
        (0..self.count()).any(|p| matches!(self.field(p), "annotation" | "returns"))
    }
}

/// The buffers `rows` works in, kept between files so a file after the first allocates nothing.
#[derive(Default)]
struct Scratch {
    /// The class bits of each element that is an annotation root.
    class: Vec<Option<u8>>,
    /// The roots of the subtrees that have their own evaluation.
    bodies: Vec<u32>,
    /// Whether each element is inside such a subtree.
    excluded: Vec<bool>,
    /// The nearest root above each element.
    nearest: Vec<u32>,
    /// Whether the nearest root of each element is known.
    done: Vec<bool>,
    /// The ancestors whose answer is the one being found.
    chain: Vec<u32>,
    /// The comprehension clauses of one generator.
    clauses: Vec<u32>,
}

thread_local! {
    /// The scratch of this thread.
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

/// What the file's state of the annotations future and the target's version make of annotations: the future is `Some(true)`
/// when the file postpones them, `Some(false)` when it states it does not, none when it states nothing.
#[must_use]
pub(crate) fn target(future: Option<bool>, major: i64, minor: i64) -> Target {
    Target {
        known: future.is_some(),
        enabled: future == Some(true),
        disabled: future == Some(false),
        native: major == 3 && minor >= 14,
        immediate: major == 3 && minor < 14,
    }
}

/// The rows of the file: every element inside an annotation with the annotation it belongs to, when it is evaluated and the mode.
/// `embedded` are the roots of the expressions read from strings.
pub(crate) fn rows(
    shape: &impl Shape,
    target: &Target,
    embedded: impl IntoIterator<Item = Position>,
    emit: &mut dyn FnMut(Row),
) {
    SCRATCH.with(|scratch| rows_in(shape, target, embedded, emit, &mut scratch.borrow_mut()));
}

/// `rows` over the buffers it keeps.
fn rows_in(
    shape: &impl Shape,
    target: &Target,
    embedded: impl IntoIterator<Item = Position>,
    emit: &mut dyn FnMut(Row),
    scratch: &mut Scratch,
) {
    if !target.known && !target.enabled {
        // no class but `unevaluated` local (needs a known feature row), postponed (enabled), eager / deferred (disabled)
        return;
    }
    let n = shape.count();
    let is_function = |k: &str| matches!(k, "function-def" | "async-function-def");
    let kind = |p: u32| shape.kind(p);
    let field = |p: u32| shape.field(p);
    let parent = |p: u32| {
        let q = shape.parent(p);
        (p != 0 && q != p).then_some(q)
    };
    let children = |x: u32| shape.children(x).filter(move |c| *c < n);
    let start = |p: u32| shape.start(p);
    // root classes: bit 0 eager, 1 postponed, 2 deferred, 3 unevaluated lazy, 4 unevaluated eager
    let Scratch { class, bodies, excluded, nearest, done, chain, clauses } = scratch;
    class.clear();
    class.resize(n as usize, None);
    bodies.clear();
    for p in 0..n {
        let k = kind(p);
        if k == "arg" || is_function(k) {
            for c in children(p) {
                let f = field(c);
                let signature = if k == "arg" {
                    // a parameter annotation whose parameter is inside a function
                    f == "annotation" && {
                        let mut at = parent(p);
                        while let Some(a) = at {
                            if is_function(kind(a)) {
                                break;
                            }
                            at = parent(a);
                        }
                        at.is_some()
                    }
                } else {
                    f == "returns"
                };
                if signature {
                    let mut bits = 0;
                    if target.disabled && target.immediate {
                        bits |= 1;
                    }
                    if target.enabled {
                        bits |= 2;
                    }
                    if target.disabled && target.native {
                        bits |= 4;
                    }
                    if let Some(slot) = class.get_mut(c as usize) {
                        *slot.get_or_insert(0) |= bits;
                    }
                }
            }
        } else if k == "ann-assign" {
            let mut scope = parent(p);
            while let Some(s) = scope {
                let sk = kind(s);
                if sk == MODULE || sk == "class-def" || sk == "lambda" || is_function(sk) {
                    break;
                }
                scope = parent(s);
            }
            let scope_kind = scope.map(&kind);
            let local = scope_kind.is_some_and(|sk| sk == "lambda" || is_function(sk));
            let outer = scope_kind.is_some_and(|sk| sk == MODULE || sk == "class-def");
            let simple = children(p).any(|t| field(t) == "target" && kind(t) == "name" && start(t) == start(p));
            for c in children(p) {
                if field(c) != "annotation" {
                    continue;
                }
                let mut bits = 0;
                if outer {
                    if target.disabled && target.immediate {
                        bits |= 1;
                    }
                    if simple {
                        if target.enabled {
                            bits |= 2;
                        }
                        if target.disabled && target.native {
                            bits |= 4;
                        }
                    } else if target.enabled || (target.disabled && target.native) {
                        bits |= 8;
                    }
                }
                if local && target.known {
                    if target.enabled || (target.disabled && target.native) {
                        bits |= 8;
                    } else {
                        bits |= 16;
                    }
                }
                // a root with no class still holds its members (they take no row)
                if let Some(slot) = class.get_mut(c as usize) {
                    *slot.get_or_insert(0) |= bits;
                }
            }
        } else if k == "lambda" {
            bodies.extend(children(p).filter(|c| field(*c) == "body"));
        } else if k == "generator-exp" {
            bodies.extend(children(p).filter(|c| field(*c) == "elt"));
            clauses.clear();
            clauses.extend(children(p).filter(|c| field(*c) == "generators"));
            let first = clauses.iter().map(|c| start(*c)).min();
            for c in clauses.iter() {
                for cc in children(*c) {
                    if !(Some(start(*c)) == first && field(cc) == "iter") {
                        bodies.push(cc);
                    }
                }
            }
        }
    }
    bodies.extend(embedded.into_iter().map(|root| root.0));
    if class.iter().flatten().all(|b| *b == 0) {
        return;
    }
    excluded.clear();
    excluded.resize(n as usize, false);
    // The roots of the subtrees with their own evaluation; their members follow through their parents, which precede them.
    for b in bodies.iter().copied() {
        if b < n {
            excluded[b as usize] = true;
        }
    }
    let mut write = |x: u32, root: u32, bits: u8| {
        for (i, (evaluation, mode)) in CLASSES.iter().enumerate() {
            if bits & (1 << i) != 0 {
                emit(Row { id: Position(x), root: Position(root), evaluation, mode });
            }
        }
    };
    // each element: its own root (when it is one) and its nearest root above it
    nearest.clear();
    nearest.resize(n as usize, u32::MAX);
    done.clear();
    done.resize(n as usize, false);
    for x in 0..n {
        // the nearest root strictly above x: from the parent's answer when it is known, else up the chain
        chain.clear();
        let mut at = parent(x);
        let found = match at {
            None => u32::MAX,
            Some(a) if class.get(a as usize).is_some_and(Option::is_some) => a,
            Some(a) if done[a as usize] => nearest[a as usize],
            Some(_) => loop {
                let Some(a) = at else { break u32::MAX };
                if class.get(a as usize).is_some_and(Option::is_some) {
                    break a;
                }
                if done[a as usize] {
                    break nearest[a as usize];
                }
                chain.push(a);
                at = parent(a);
            },
        };
        for c in chain.iter().copied() {
            nearest[c as usize] = found;
            done[c as usize] = true;
        }
        nearest[x as usize] = found;
        done[x as usize] = true;
        if parent(x).is_some_and(|above| excluded[above as usize]) {
            excluded[x as usize] = true;
        }
        if excluded[x as usize] {
            continue;
        }
        if found != u32::MAX {
            write(x, found, class[found as usize].unwrap_or(0));
        }
        if let Some(bits) = class.get(x as usize).and_then(Option::as_ref) {
            write(x, x, *bits);
        }
    }
}

//! The one-worker harness runs the optional predicate and the executor for one
//! file consecutively. The file's feedback channel expires after this handoff.
use super::{learning::{Learning, Shared, WINDOW}, prices, cost::Family};
use std::{cell::RefCell, rc::{Rc, Weak}};

#[derive(Default)]
struct Handoff {
    target: Weak<RefCell<Learning>>,
    path: Vec<u8>,
}
thread_local! { static FILE: RefCell<Handoff> = RefCell::default(); }

pub(crate) fn send(target:&Shared, path:Option<&[u8]>) {
    FILE.with_borrow_mut(|f| {
        f.target = if path.is_some() { Rc::downgrade(target) } else { Weak::new() };
        f.path.clear();
        if let Some(path) = path { f.path.extend_from_slice(path); }
    });
}

pub(crate) fn receive(path:&str, files:usize)->Option<Shared> {
    FILE.with_borrow_mut(|f| {
        let target=std::mem::take(&mut f.target).upgrade()?;
        if path.as_bytes()!=f.path { return None; }
        if (files as u64)<WINDOW { target.borrow_mut().active=false; return None; }
        Some(target)
    })
}

pub(crate) fn lookup_price()->u64 { prices::LOGIC }
pub(crate) fn work(units:u64, provider:u64)->u64 {
    lookup_price().saturating_add(units).saturating_add(provider)
}
pub(crate) fn operation(class:Family)->u64 {
    use Family::*;
    match class {
        Logic => prices::LOGIC, Shape => prices::CELL,
        Text | Filename | Sets => prices::TEXT, Parent | Child => prices::TREE,
        Count => prices::COUNT, _ => prices::PROBE,
    }
}
pub(crate) fn supplier(family:Family)->u64 { prices::provider(family) }


//! Audit the select front's separate entry-list slots together with complete view lists.
//! The map records order; decoding a select's bucket happens after the file finished.
use super::cold_list::{Metric, conjuncts};
use super::{Exec, File, Native, Run, ViewSpec, bucket, kind_set, lit_pick};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::{Cell, RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use super::cold_bucket::Slot;
type Map = FxHashMap<usize, Slot>;
pub(super) type Key = (Native, Option<(usize, Vec<u32>)>, Vec<super::cold_list::FilterKey>);
const SHIFT: usize = 56;
// Namespace tags of the two positive filtered driver-slot keys in select_kernel.
const KIND_ENTRY: usize = 3;
const LITERAL_ENTRY: usize = 4;
const MASK: usize = (1 << SHIFT) - 1;
#[derive(Default)]
pub(super) struct Rows {
    data: Map,
    owner: Option<usize>,
    filtering_entry: Cell<bool>,
    pending: Cell<(usize, f64)>,
    events: RefCell<Vec<Event>>,
}
#[derive(Clone, Copy)]
enum Event {
    Read(usize, usize),
    Build(usize, usize, Metric),
}
impl Deref for Rows {
    type Target = Map;
    fn deref(&self) -> &Map {
        &self.data
    }
}
impl DerefMut for Rows {
    fn deref_mut(&mut self) -> &mut Map {
        &mut self.data
    }
}
impl Rows {
    #[inline]
    pub(super) fn take_filtering_entry(&self) -> bool {
        if self.filtering_entry.get() {
            self.filtering_entry.set(false);
            true
        } else {
            false
        }
    }
    pub(super) fn owner(&mut self, owner: Option<usize>) {
        self.owner = owner;
    }
    #[inline]
    pub(super) fn get(&self, key: &usize) -> Option<&Rc<Vec<u32>>> {
        self.slot(key).map(Slot::owned)
    }
    pub(super) fn slot(&self, key: &usize) -> Option<&Slot> {
        let rows = self.data.get(key);
        // Only the two cached driver-entry constructors next call view_cols::retain.
        // Transient per-key join hits use that function too, outside this scope.
        if matches!(*key >> SHIFT, KIND_ENTRY | LITERAL_ENTRY) && rows.is_none() {
            self.filtering_entry.set(true);
        }
        if sw!("VEC_COLD_ENTRIES") {
            if rows.is_some() {
                self.events
                    .borrow_mut()
                    .push(Event::Read(*key, self.owner.unwrap_or(*key)));
            } else {
                self.pending.set((*key, super::cold_list::clock()));
            }
        }
        rows
    }
    #[inline]
    pub(super) fn insert(&mut self, key: usize, rows: impl Into<Slot>) -> Option<Slot> {
        let rows = rows.into();
        if sw!("VEC_COLD_ENTRIES") && key >> SHIFT < 5 {
            let (k, start) = self.pending.get();
            assert_eq!(k, key, "nested cold list");
            let m = Metric {
                builds: 1,
                entered: 0,
                tests: 0,
                kept: rows.len() as u64,
                seconds: super::cold_list::clock() - start,
            };
            self.events
                .get_mut()
                .push(Event::Build(key, self.owner.unwrap_or(key), m));
        }
        self.data.insert(key, rows)
    }
    pub(super) fn primary(&mut self, key: usize, entered: usize, tests: u64) {
        if !sw!("VEC_COLD_ENTRIES") {
            return;
        }
        let Some(Event::Build(k, _, m)) = self.events.get_mut().last_mut() else {
            panic!("missing whole view")
        };
        assert_eq!(*k, key);
        m.entered = entered as u64;
        m.tests = tests;
    }
}

struct Desc {
    id: usize,
    view: Option<Rc<ViewSpec>>,
    kinds: Vec<u32>,
    literal: Option<(u16, usize, Vec<u32>)>,
}
#[derive(Default)]
pub(super) struct Audit {
    ids: FxHashMap<Key, usize>,
    slots: FxHashMap<usize, Desc>,
    sums: std::collections::BTreeMap<(usize, bool), Metric>,
    fan: std::collections::BTreeMap<usize, u64>,
}
fn identity(base: Native, mut entry: Option<(usize, Vec<u32>)>, tests: &[super::E]) -> Key {
    if let Some((_, xs)) = &mut entry {
        xs.sort_unstable();
        xs.dedup();
    }
    let mut residual = Vec::new();
    for test in tests {
        if let Some((col, xs)) = &entry
            && let Some(accepted) = super::cold_list::members(test, *col)
            && xs.iter().all(|x| accepted.contains(x))
        {
            continue;
        }
        residual.extend(conjuncts(std::slice::from_ref(test)));
    }
    residual.sort_unstable();
    residual.dedup();
    (base, entry, residual)
}

pub(super) fn view_key(run: &mut Run, v: &ViewSpec) -> Key {
    let entry = match v.base {
        Native::Tree(_) => kind_set(run, &v.filter, 0, v.base, &[]).map(|xs| (1, xs)),
        Native::Fact(i) => {
            super::leaf_scan::fact_entry(run, i, &v.filter).map(|(c, _, xs)| (c, xs))
        }
        _ => None,
    };
    identity(v.base, entry, &v.filter)
}
impl Audit {
    fn decode(&mut self, exec: &mut Exec<'_>, run: &mut Run, f: &File<'_>, key: usize) {
        if self.slots.contains_key(&key) {
            return;
        }
        let ns = key >> SHIFT;
        let low = key & MASK;
        let mut kinds = Vec::new();
        let mut literal = None;
        let mut filter = None;
        let identity = if ns == 0 {
            let v = exec.view(run, low).expect("view").expect("view node");
            view_key(run, &v)
        } else {
            let (v, projection) = if ns == 1 {
                let v = exec.view(run, low).expect("filter").expect("view");
                literal = lit_pick(f, &v.filter, 0, v.base, &[]);
                (v, false)
            } else {
                let side = low & 1;
                let sp = exec.sel(run, low >> 1).expect("executed select");
                let v = exec
                    .view(run, sp.srcs[side])
                    .expect("source")
                    .expect("view");
                if ns == KIND_ENTRY {
                    kinds = kind_set(run, &sp.conj, side, v.base, &v.map).expect("kind entry");
                } else {
                    literal = lit_pick(f, &sp.conj, side, v.base, &v.map);
                }
                (v, ns >= KIND_ENTRY)
            };
            let entry = if ns == KIND_ENTRY {
                Some((1, kinds.clone()))
            } else {
                literal.as_ref().map(|(_, c, xs)| (*c, xs.clone()))
            };
            let tests = if projection {
                filter = Some(v.clone());
                v.filter.as_slice()
            } else {
                &[]
            };
            identity(v.base, entry, tests)
        };
        let next = self.ids.len();
        let id = *self.ids.entry(identity).or_insert(next);
        let _ = self.slots.insert(
            key,
            Desc {
                id,
                view: filter,
                kinds,
                literal,
            },
        );
    }

    fn count_entry(
        d: &Desc,
        f: &File<'_>,
        run: &mut Run,
        key: usize,
        actual: &[u32],
        m: &mut Metric,
    ) {
        let ns = key >> SHIFT;
        if ns == 0 {
            return;
        }
        if ns <= 2 {
            let (i, col, values) = d.literal.as_ref().expect("fact entry");
            let ix = f.fact_list(*i, *col);
            m.entered = values
                .iter()
                .filter_map(|s| ix.get(&u64::from(*s)))
                .map(|r| r.len() as u64)
                .sum();
        } else {
            let mut rows = if ns == KIND_ENTRY {
                bucket(f, run, &d.kinds, d.view.as_ref().map_or(super::Extent::Whole, |v| v.base.extent()))
            } else {
                f.vrows.data[&((2 << SHIFT) | (key & MASK))]
                    .owned().as_ref()
                    .clone()
            };
            m.entered = rows.len() as u64;
            let v = d.view.as_ref().expect("view filter");
            m.tests = super::leaf_scan::count_tests(f, run, v, key, false, &mut rows);
            assert_eq!(rows, actual, "select entry {key}");
        }
    }

    pub(super) fn finish(&mut self, exec: &mut Exec<'_>, run: &mut Run, f: &mut File<'_>) {
        let events = std::mem::take(f.vrows.events.get_mut());
        let mut seen: FxHashMap<usize, (usize, FxHashSet<usize>)> = FxHashMap::default();
        let mut file = [Metric::default(); 2];
        let mut events = events;
        for event in events.drain(..) {
            let (key, owner) = match event {
                Event::Read(k, o) | Event::Build(k, o, _) => (k, o),
            };
            self.decode(exec, run, f, key);
            let d = &self.slots[&key];
            let dup = seen.contains_key(&d.id);
            let (first, owners) = seen
                .entry(d.id)
                .or_insert_with(|| (key, FxHashSet::default()));
            let _ = owners.insert(owner);
            if let Event::Build(_, _, mut m) = event {
                if sw!("VEC_COLD_COUNT") {
                    let actual = f.vrows.data[&key].resolve(f.table).shadow();
                    Self::count_entry(d, f, run, key, &actual, &mut m);
                    assert_eq!(
                        f.vrows.data[first].resolve(f.table).shadow(),
                        actual.as_slice(),
                        "cross-entry identity {key}"
                    );
                }
                self.sums.entry((key >> SHIFT, dup)).or_default().add(m);
                file[usize::from(dup)].add(m);
            }
        }
        for (_, owners) in seen.values() {
            *self.fan.entry(owners.len()).or_default() += 1;
        }
        if sw!("VEC_COLD_FILES") {
            println!(
                "COLD_ALL_FILE\t{}\t{}\t{}\t{}",
                f.file,
                seen.len(),
                file[0].tsv(),
                file[1].tsv()
            );
        }
        *f.vrows.events.get_mut() = events;
    }
    pub(super) fn report(&self) {
        for ((ns, dup), m) in &self.sums {
            println!("COLD_ALL\t{ns}\t{dup}\t{}", m.tsv());
        }
        for (n, count) in &self.fan {
            println!("COLD_ALL_FAN\t{n}\t{count}");
        }
    }
}

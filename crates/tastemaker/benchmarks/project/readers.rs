//! Close each ordered file population once and release its blocks after the last
//! known consumer. A live host can append new requests in another epoch.
use super::{Work, finish, session};
use finish::{partition::Partition, population::Inputs};
use std::{collections::HashMap, rc::Rc};

// While Closed is being prepared, Work holds every allocation alive. The
// pointer shortcut avoids hashing the same list again. Separate allocations
// compare by their complete, ordered contents, including repetitions.
#[derive(Default)]
struct PathLists<'a> {
    allocations: HashMap<*const Vec<String>, usize>,
    lists: HashMap<&'a [String], usize>,
}

impl<'a> PathLists<'a> {
    fn id(&mut self, paths: &'a Rc<Vec<String>>) -> usize {
        let next = self.lists.len();
        *self.allocations.entry(Rc::as_ptr(paths)).or_insert_with(|| {
            *self.lists.entry(paths.as_slice()).or_insert(next)
        })
    }
}

pub(super) struct Closed {
    inventory: Vec<usize>,
    numbers: Vec<Option<Vec<u32>>>,
    pub(super) duplicate: Vec<Option<usize>>,
    reads: Vec<Option<usize>>,
    last: HashMap<usize, usize>,
    remaining: Vec<usize>,
    providers: HashMap<usize, usize>,
    shared: HashMap<usize, Inputs>,
    keep: bool,
}
impl Closed {
    pub(super) fn new(work: &[Work], cut_of: &[Option<std::sync::Arc<Partition>>], keep: bool) -> Self {
        // Slots name a canonical source version and binding. Order and repetitions
        // are part of the population identity. A live host can open another epoch.
        let mut lists = PathLists::default();
        let mut inventory_ids = vec![0; work.len()];
        let mut owners = HashMap::new();
        let mut duplicate = vec![None; work.len()];
        let mut scopes = HashMap::new();
        let mut reads = vec![None; work.len()];
        let mut last = HashMap::<usize, usize>::new();
        let mut remaining = Vec::<usize>::new();
        let mut providers = HashMap::<usize, usize>::new();
        for (i, work) in work.iter().enumerate() {
            let Work::Closed {
                pairs,
                binding,
                project,
                inventory,
                files,
            } = work
            else {
                continue;
            };
            // Answer equality also depends on admission and host values. Scope
            // identifiers carry their authority and input-version inventory.
            inventory_ids[i] = lists.id(inventory);
            let owner = *owners
                .entry((
                    *binding,
                    *project,
                    pairs.as_slice(),
                    inventory_ids[i],
                    lists.id(files),
                ))
                .or_insert(i);
            if owner != i {
                duplicate[i] = Some(owner);
            }
            if let Some(p) = project {
                *providers.entry(*p).or_default() += 1;
            }
            let Some(cut) = &cut_of[*binding] else {
                continue;
            };
            let next = scopes.len();
            let id = *scopes
                .entry((std::sync::Arc::as_ptr(cut), pairs.as_slice()))
                .or_insert(next);
            if id == remaining.len() {
                remaining.push(0);
            }
            remaining[id] += 1;
            reads[i] = Some(id);
            if !keep {
                for &slot in pairs {
                    *last.entry(slot).or_default() += 1;
                }
            }
        }
        drop(scopes);
        drop(owners);

        Self {
            numbers: vec![None; lists.lists.len()],
            inventory: inventory_ids,
            duplicate,
            reads,
            last,
            remaining,
            providers,
            shared: HashMap::new(),
            keep,
        }
    }
    pub(super) fn inventory_id(&self, request: usize) -> usize {
        self.inventory[request]
    }
    pub(super) fn numbered_inventories(&self) -> &[Option<Vec<u32>>] {
        &self.numbers
    }
    pub(super) fn numbers(
        &mut self,
        request: usize,
        paths: &[String],
        world: &mut session::World,
    ) -> &[u32] {
        self.numbers[self.inventory[request]]
            .get_or_insert_with(|| paths.iter().map(|path| world.file(path)).collect())
    }
    pub(super) fn inputs(
        &mut self,
        i: usize,
        make: impl FnOnce() -> Result<Inputs, String>,
    ) -> Result<Inputs, String> {
        let id = self.reads[i].expect("registered closed input");
        Ok(match self.shared.get(&id) {
            Some(prior) => finish::population::Inputs::clone(prior),
            None => {
                let inputs = make()?;
                if self.remaining[id] > 1 {
                    self.shared.insert(id, inputs.clone());
                }
                inputs
            }
        })
    }
    pub(super) fn release(
        &mut self,
        i: usize,
        pairs: &[usize],
        project: Option<usize>,
        world: &mut session::World,
    ) -> Option<usize> {
        if let Some(id) = self.reads[i] {
            self.remaining[id] -= 1;
            if self.remaining[id] == 0 {
                self.shared.remove(&id);
            }
            if !self.keep {
                for slot in pairs {
                    let n = self.last.get_mut(slot).expect("registered file input");
                    *n -= 1;
                    if *n == 0 {
                        world.release(*slot);
                    }
                }
            }
        }
        if !self.keep
            && let Some(p) = &project
        {
            let n = self
                .providers
                .get_mut(p)
                .expect("registered provider input");
            *n -= 1;
            if *n == 0 {
                return Some(*p);
            }
        }
        None
    }
}

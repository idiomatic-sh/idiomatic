//! Close each ordered file population once and release its blocks after the last
//! reader in the admitted epoch.
use super::{Work, finish, session};
use finish::{partition::Partition, population::Inputs};
use std::{collections::HashMap, rc::Rc, sync::Arc};

// While Closed is being prepared, Work holds every allocation alive. The
// pointer shortcut avoids hashing the same list again. Separate allocations
// compare by their complete, ordered contents, including repetitions.
#[derive(Default)]
pub struct PathLists<'a> {
    pub allocations: HashMap<*const Vec<String>, usize>,
    pub lists: HashMap<&'a [String], usize>,
}

impl<'a> PathLists<'a> {
    pub fn id(&mut self, paths: &'a Rc<Vec<String>>) -> usize {
        let next = self.lists.len();
        *self.allocations.entry(Rc::as_ptr(paths)).or_insert_with(|| {
            *self.lists.entry(paths.as_slice()).or_insert(next)
        })
    }
}

pub struct Closed {
    pub scopes: HashMap<usize, [usize; 3]>,
    pub repeats: HashMap<[usize; 3], usize>,
    pub number_wall: f64,
    pub numbered: usize,
    pub numbers: Vec<Option<Vec<u32>>>,
    pub duplicate: Vec<Option<usize>>,
    pub reads: Vec<Option<usize>>,
    pub last: HashMap<usize, usize>,
    pub remaining: Vec<usize>,
    pub providers: HashMap<usize, usize>,
    pub shared: HashMap<usize, Inputs>,
}
impl Closed {
    pub fn new(work: &[Work], cut_of: &[Option<Arc<Partition>>]) -> Self {
        // Slots name a canonical source version and binding. Order and repetitions
        // are part of the population identity. A live host can open another epoch.
        let mut lists = PathLists::default();
        let mut request_scopes = HashMap::new();
        let mut repeats = HashMap::new();
        let mut owners = HashMap::new();
        let mut duplicate = vec![None; work.len()];
        let mut scopes = HashMap::new();
        let mut reads = vec![None; work.len()];
        let mut last = HashMap::<usize, usize>::new();
        let mut remaining = Vec::<usize>::new();
        let mut providers = HashMap::<usize, usize>::new();
        for (i, work) in work.iter().enumerate() {
            let Work::Closed {
                root, pairs,
                binding,
                project,
                inventory,
                files,
                ..
            } = work
            else {
                continue;
            };
            // Answer equality also depends on admission and host values. Scope
            // identifiers carry their authority and input-version inventory.
            let scope = [*binding, lists.id(inventory), lists.id(files)];
            request_scopes.insert(i, scope);
            let owner = *owners
                .entry((
                    scope, root.as_ref(),
                    *project,
                    pairs.as_slice(),
                ))
                .or_insert(i);
            if owner != i {
                duplicate[i] = Some(owner);
            } else {
                *repeats.entry(scope).or_insert(0) += 1;
            }
            if let Some(p) = project {
                *providers.entry(*p).or_default() += 1;
            }
            let Some(cut) = &cut_of[*binding] else {
                continue;
            };
            let next = scopes.len();
            let id = *scopes
                .entry((Arc::as_ptr(cut), pairs.as_slice()))
                .or_insert(next);
            if id == remaining.len() {
                remaining.push(0);
            }
            remaining[id] += 1;
            reads[i] = Some(id);
            for &slot in pairs {
                *last.entry(slot).or_default() += 1;
            }
        }
        drop(scopes);
        drop(owners);

        Self {
            numbers: vec![None; lists.lists.len()],
            scopes: request_scopes,
            repeats,
            number_wall: 0.,
            numbered: 0,
            duplicate,
            reads,
            last,
            remaining,
            providers,
            shared: HashMap::new(),
        }
    }
    pub fn scope(&self, request: usize) -> [usize; 3] { self.scopes[&request] }

    pub fn may_repeat(&self, scope: [usize; 3]) -> bool {
        self.repeats.get(&scope).is_some_and(|&n| n > 1)
    }

    pub fn inventory_id(&self, request: usize) -> usize {
        self.scopes[&request][1]
    }
    pub fn numbered_inventories(&self) -> &[Option<Vec<u32>>] {
        &self.numbers
    }
    pub fn numbers(
        &mut self,
        request: usize,
        paths: &[String],
        world: &mut session::World,
    ) -> &[u32] {
        self.numbers[self.scopes[&request][1]].get_or_insert_with(|| {
            let start = std::time::Instant::now();
            let numbers = paths.iter().map(|path| world.file(path)).collect();
            self.number_wall += start.elapsed().as_secs_f64();
            self.numbered += paths.len();
            numbers
        })
    }
    pub fn inputs(
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
    pub fn release(
        &mut self,
        i: usize,
        pairs: &[usize],
        project: Option<usize>,
        mut release_pair: impl FnMut(usize),
    ) -> Option<usize> {
        if let Some(id) = self.reads[i] {
            self.remaining[id] -= 1;
            if self.remaining[id] == 0 {
                self.shared.remove(&id);
            }
            for slot in pairs {
                let n = self.last.get_mut(slot).expect("registered file input");
                *n -= 1;
                if *n == 0 {
                    self.last.remove(slot);
                    release_pair(*slot);
                }
            }
        }
        if let Some(p) = project {
            let n = self
                .providers
                .get_mut(&p)
                .expect("registered provider input");
            *n -= 1;
            if *n == 0 {
                self.providers.remove(&p);
                return Some(p);
            }
        }
        None
    }

    /// The worker scope has joined and no borrowed frontier remains.
    pub fn release_finished(
        &mut self,
        mut release_pair: impl FnMut(usize),
        mut release_population: impl FnMut(usize),
    ) {
        self.shared.clear();
        for (slot, _) in self.last.drain() {
            release_pair(slot);
        }
        for (project, _) in self.providers.drain() {
            release_population(project);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_frontier_lives_until_its_last_closed_reader() {
        let paths = Rc::new(Vec::new());
        let request = |pairs| Work::Closed {
            pairs,
            binding: 0,
            root: "/".into(),
            project: Some(7),
            inventory: paths.clone(),
            files: paths.clone(),
            absent: Default::default(),
        };
        let requests = [request(vec![0, 1, 1]), request(vec![0, 2])];
        let cut = Some(Arc::new(Partition::default()));
        let mut readers = Closed::new(&requests, &[cut]);
        let mut world = session::World::default();
        for slot in 0..3 {
            world.restore_frontier(slot, Vec::new());
        }

        assert_eq!(readers.release(0, &[0, 1, 1], Some(7), |slot| world.release(slot)), None);
        assert!(world.frontier_parts(0).is_some());
        assert!(world.frontier_parts(1).is_none());
        assert!(world.frontier_parts(2).is_some());

        let mut populations = Vec::new();
        readers.release_finished(|slot| world.release(slot), |id| populations.push(id));
        assert_eq!(populations, vec![7]);
        assert!(world.frontier_parts(0).is_none());
        assert!(world.frontier_parts(2).is_none());
    }
}

/// Compare complete typed contents only for scopes that can actually repeat.
/// Bound retention to small inputs and outputs of this prepared program.
#[derive(Default)]
pub struct Evaluated {
    pub entries: HashMap<[usize; 3], Vec<Evaluation>>,
    pub cells: usize,
    pub hits: usize,
}

pub struct Evaluation {
    pub project: Option<std::collections::BTreeMap<String, Rc<finish::Table>>>,
    pub inputs: Inputs,
    pub answer: session::Reported,
}

impl Evaluated {
    pub fn find(
        &mut self,
        scope: [usize; 3],
        project: Option<&super::Population>,
        inputs: &Inputs,
    ) -> Option<session::Reported> {
        let project = project.map(|p| &p.tables);
        let match_ = self.entries.get(&scope)?.iter().find(|entry| {
            entry.inputs.len() == inputs.len()
                && entry.project.as_ref() == project
                && entry.inputs.iter().zip(inputs).all(|((slot, a),(other, b))| {
                    slot == other && a.same_rows(b)
                })
        })?;
        self.hits += 1;
        Some(match_.answer.clone())
    }

    pub fn remember(
        &mut self,
        scope: [usize; 3],
        project: Option<&super::Population>,
        inputs: &Inputs,
        answer: &session::Reported,
    ) {
        let cells = inputs.values().map(|p| p.cells()).sum::<usize>()
            + project.iter().flat_map(|p| p.tables.values()).map(|t| t.n.saturating_mul(t.cols.len().max(1))).sum::<usize>();
        if cells > 4096 || self.cells.saturating_add(cells.max(1)) > 16384 || answer.rows.len() > 32 { return; }
        self.cells += cells.max(1);
        self.entries.entry(scope).or_default().push(Evaluation {
            project: project.map(|p| p.tables.clone()),
            inputs: inputs.clone(),
            answer: answer.clone(),
        });
    }
}

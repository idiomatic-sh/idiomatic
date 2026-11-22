//! The `selection_pushing` service: free functions named by verb over the filters of a step made on the inputs it reads.

pub(crate) mod factoring;
pub(crate) mod joins;
pub(crate) mod placing;

use self::factoring::service::{shared_into, shared_push};
use self::joins::service::pushed_join;
use self::placing::service::{filter_of, finished, insert_before, insert_ordered, place_after};
use super::types::PushedFilter;
use super::types::StepHash;
use super::types::{PlanSplice, Spliced};
use crate::domain::pruning::service::without_unused_roots;
use crate::domain::rewriting::types::Planned;
use crate::domain::rewriting::types::StepIndex;
use crate::domain::widths::service::width_of;
use ryo_values::domain::plan_parts::service::fixed_prefix;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::RowWidth;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::{each_input, inputs_of, named};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use std::hash::{BuildHasherDefault, Hasher};

/// The plan with each filter of a join placed on the join's inputs, and the filters of a union made on both its inputs: the join or
/// the union combines only the rows the filter lets through. A filter of a join moves the comparisons of its infallible prefix
/// (`fixed_prefix`) that test one input: one of the left is a filter of the left; one of the right rides the probe when the probe
/// walks the tree by a node (`rides_the_walk`), and is a filter of the right otherwise; one that tests only keys of the join is
/// also made on the other input, whose keys hold the same values; an equality of a left and a right column of one kind a key
/// compares alike is a key of the join. Each filter of a join is placed on its own copy of the join, and a reader of the join that
/// is no filter keeps reading the join. A union's filters move, their infallible prefix alone, only when every reader of the union
/// is such a filter. When every reader of an antijoin, a projection, an extension or a filter is a filter of it, the comparisons
/// of their infallible prefixes that all of them make are made below it once, with no copy (`shared_push`): on an antijoin's left,
/// and on its right when they test only keys; on what a projection, an extension (the columns it reads alone) or a filter reads.
/// The rest of a filter stays after the step, in its order, so a comparison that may fail runs on the rows it ran on:
/// `σ(L ⋈ R) = σL ⋈ σR`, `σ(A ∪ B) = σA ∪ σB` and `σ(L ▷ R) = σL ▷ σR`. Done again, until no filter moves, each pass making a
/// repeated step once (`without_repeats`); each move takes a filter strictly down, so it ends.
#[must_use]
pub(crate) fn push_selections(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = push_selections_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn push_selections_roots(
    mut steps: Vec<PlanStep>,
    roots: &mut [StepId],
) -> Vec<PlanStep> {
    let mut readers = Readers::default();
    loop {
        let mut moved = false;
        // A plan whose every step reads only steps before it is renumbered from the splice on, and its readers are kept
        // as it changes; any other plan is renumbered whole and its readers read again after each push.
        let ordered = in_reading_order(&steps);
        let splice: PlanSplice = if ordered {
            insert_ordered
        } else {
            insert_before
        };
        readers.rebuild(&steps);
        for place in (0..steps.len()).rev() {
            let id = StepId(u32::try_from(place).unwrap_or(u32::MAX));
            if roots.contains(&id) {
                continue;
            }
            if let Some(push) = readers
                .get(place)
                .and_then(|reading| shared_push(&steps, id, reading))
            {
                let spliced = shared_into(&mut steps, roots, push, splice);
                match spliced {
                    Some(spliced) if ordered => readers.spliced(&steps, &spliced, place),
                    Some(_) | None => readers.rebuild(&steps),
                }
                moved = true;
                continue;
            }
            let pushed = readers
                .get(place)
                .map(|reading| placed(&steps, id, reading))
                .unwrap_or_default();
            if pushed.is_empty() {
                continue;
            }
            // The last reader first, so the places of the ones before it stay as they are.
            for PushedFilter {
                reader,
                before,
                filter,
            } in pushed.into_iter().rev()
            {
                let spliced = Spliced {
                    at: reader,
                    count: before.len(),
                    replaced: Vec::new(),
                };
                splice(&mut steps, roots, reader, before, filter);
                if ordered {
                    readers.spliced(&steps, &spliced, place);
                }
            }
            if !ordered {
                readers.rebuild(&steps);
            }
            moved = true;
        }
        if !moved {
            return steps;
        }
        steps = without_repeats(without_unused_roots(steps, roots), roots);
    }
}

/// Whether every step of a plan reads only steps that stand before it.
fn in_reading_order(steps: &[PlanStep]) -> bool {
    steps.iter().enumerate().all(|(place, step)| {
        let inputs = inputs_of(&step.operation);
        named(&inputs).iter().all(|input| match input {
            Input::Step(id) => usize::try_from(id.0).is_ok_and(|read| read < place),
            _ => true,
        })
    })
}

/// The hasher a repeated step is looked up by: a rotate, an exclusive or and a multiply a word. A hash only names the few
/// steps a step is compared with in full, so any hasher gives the same plan; this one is several times cheaper than the
/// standard library's keyed one, and every step of a plan is hashed once a pass.
#[derive(Default)]
struct StepHasher(u64);

impl StepHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl Hasher for StepHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0; 8];
            for (slot, byte) in word.iter_mut().zip(chunk) {
                *slot = *byte;
            }
            self.add(u64::from_le_bytes(word));
        }
    }

    fn write_u8(&mut self, word: u8) {
        self.add(u64::from(word));
    }

    fn write_u16(&mut self, word: u16) {
        self.add(u64::from(word));
    }

    fn write_u32(&mut self, word: u32) {
        self.add(u64::from(word));
    }

    fn write_u64(&mut self, word: u64) {
        self.add(word);
    }

    fn write_usize(&mut self, word: usize) {
        self.add(u64::try_from(word).unwrap_or(u64::MAX));
    }
}

/// The plan with each step that repeats an earlier one (the same operation on the same inputs, made where it is) read from the
/// earlier one instead, each filter that tests nothing and keeps every column read as the step it reads, and every later step and
/// root renumbered: filters pushed down along paths that meet again make the same steps, and a step that only filters read is a
/// step the next push may go below. A root is never merged into another step, so each stays its own step.
fn without_repeats(steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    let mut made: Vec<PlanStep> = Vec::with_capacity(steps.len());
    let mut seen: std::collections::HashMap<StepHash, Vec<StepId>, BuildHasherDefault<StepHasher>> =
        std::collections::HashMap::default();
    let mut moved: Vec<StepId> = Vec::with_capacity(steps.len());
    for (place, mut step) in steps.into_iter().enumerate() {
        each_input(&mut step.operation, &mut |input| {
            if let Input::Step(id) = input
                && let Some(now) = moved.get(usize::try_from(id.0).unwrap_or(usize::MAX))
            {
                *id = *now;
            }
        });
        let root = roots
            .iter()
            .any(|root| usize::try_from(root.0).is_ok_and(|at| at == place));
        // A filter that tests nothing and keeps every column is the step it reads.
        if let Operation::Select {
            from: Input::Step(read),
            comparisons,
            kept,
        } = &step.operation
            && !root
            && comparisons.is_empty()
            && kept.is_empty()
            && step_in(&made, *read).is_some_and(|other| other.scope == step.scope)
        {
            moved.push(*read);
            continue;
        }
        let key = {
            let mut hasher = StepHasher::default();
            std::hash::Hash::hash(&step, &mut hasher);
            StepHash(hasher.finish())
        };
        let earlier = seen.get(&key).and_then(|ids| {
            ids.iter()
                .copied()
                .find(|id| step_in(&made, *id).is_some_and(|other| *other == step))
        });
        let id = match earlier {
            Some(id) if !root => id,
            _ => {
                let id = StepId(u32::try_from(made.len()).unwrap_or(u32::MAX));
                seen.entry(key).or_default().push(id);
                made.push(step);
                id
            }
        };
        moved.push(id);
    }
    for root in roots.iter_mut() {
        if let Some(now) = moved.get(usize::try_from(root.0).unwrap_or(usize::MAX)) {
            *root = *now;
        }
    }
    made
}

/// Spike (ryo-fmx3): for each step, the places of the steps that read it, in step order; the lists keep their room across
/// the rewrites of a plan. After a push the lists of the steps below the pushed step are brought up to date in place
/// (`spliced`): they are the only ones a pass reads again, since it goes from the last step down.
#[derive(Default)]
struct Readers {
    lists: Vec<Vec<StepIndex>>,
}

impl Readers {
    /// Gives `visit` each step `step` reads, once.
    fn each_read(step: &PlanStep, visit: &mut dyn FnMut(usize)) {
        let inputs = inputs_of(&step.operation);
        let named = named(&inputs);
        for (at, input) in named.iter().enumerate() {
            if let Input::Step(id) = *input
                && !named.get(..at).is_some_and(|before| before.contains(input))
            {
                visit(usize::try_from(id.0).unwrap_or(usize::MAX));
            }
        }
    }

    /// The readers of `steps`, each reader once per step it reads, in step order.
    fn rebuild(&mut self, steps: &[PlanStep]) {
        for list in &mut self.lists {
            list.clear();
        }
        self.lists.resize_with(steps.len(), Vec::new);
        for (place, step) in steps.iter().enumerate() {
            Self::each_read(step, &mut |read| {
                if let Some(list) = self.lists.get_mut(read) {
                    list.push(StepIndex(place));
                }
            });
        }
    }

    /// Notes the step at `place` among the readers of each step below `below` that it reads.
    fn note(&mut self, steps: &[PlanStep], place: usize, below: usize) {
        let Some(step) = steps.get(place) else {
            return;
        };
        Self::each_read(step, &mut |read| {
            if read < below
                && let Some(list) = self.lists.get_mut(read)
            {
                let at = list.partition_point(|reader| reader.0 < place);
                if list.get(at) != Some(&StepIndex(place)) {
                    list.insert(at, StepIndex(place));
                }
            }
        });
    }

    /// Brings the readers of the steps below `below` up to date after a push, for a plan in reading order: the readers from
    /// the splice on stand `count` later; the step replaced at the splice, the steps made before it and the other steps
    /// replaced read what they read now.
    fn spliced(&mut self, steps: &[PlanStep], spliced: &Spliced, below: usize) {
        let (at, count) = (spliced.at.0, spliced.count);
        for list in self.lists.iter_mut().take(below) {
            let from = list.partition_point(|reader| reader.0 < at);
            if list.get(from) == Some(&StepIndex(at)) {
                let _replaced = list.remove(from);
            }
            for reader in list.iter_mut().skip(from) {
                reader.0 += count;
            }
        }
        for place in at..=at.saturating_add(count) {
            self.note(steps, place, below);
        }
        for other in &spliced.replaced {
            for list in self.lists.iter_mut().take(below) {
                if let Ok(found) = list.binary_search_by_key(&other.0, |reader| reader.0) {
                    let _replaced = list.remove(found);
                }
            }
            self.note(steps, other.0, below);
        }
        debug_assert!(
            self.agrees(steps, below),
            "the readers kept differ from the readers read again"
        );
    }

    /// Whether the lists of the steps below `below` are what reading `steps` again gives.
    fn agrees(&self, steps: &[PlanStep], below: usize) -> bool {
        let mut again = Self::default();
        again.rebuild(steps);
        self.lists
            .iter()
            .zip(&again.lists)
            .take(below)
            .all(|(kept, read)| kept == read)
    }

    /// The readers of the step at `place`.
    fn get(&self, place: usize) -> Option<&[StepIndex]> {
        self.lists.get(place).map(Vec::as_slice)
    }
}

/// What takes the place of each of `readers`, the steps that read the union or the join `id`, whose comparisons move: for a join,
/// each filter that places some on an input; for a union, every reader, when each is a filter whose infallible prefix moves, and
/// none otherwise.
fn placed(steps: &[PlanStep], id: StepId, readers: &[StepIndex]) -> Vec<PushedFilter> {
    let Some(made) = step_in(steps, id) else {
        return Vec::new();
    };
    // The readers that are filters of the step, made where it is.
    let filtering: Vec<StepIndex> = readers
        .iter()
        .copied()
        .filter(|reader| {
            steps.get(reader.0).is_some_and(|step| {
                step.scope == made.scope
                    && matches!(step.operation, Operation::Select { from, .. } if from == Input::Step(id))
            })
        })
        .collect();
    if let Operation::Union { left, right } = &made.operation {
        if filtering.is_empty() || filtering.len() != readers.len() {
            return Vec::new();
        }
        filtering
            .iter()
            .map(|reader| pushed_union(steps, id, *reader, *left, *right))
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default()
    } else if let Operation::Join { reach: None, .. } = &made.operation {
        filtering
            .iter()
            .filter_map(|reader| pushed_join(steps, id, *reader))
            .collect()
    } else {
        Vec::new()
    }
}

/// What takes the place of the filter at `reader` of the union `id` of `left` and `right`, when its infallible prefix moves.
fn pushed_union(
    steps: &[PlanStep],
    id: StepId,
    reader: StepIndex,
    left: Input,
    right: Input,
) -> Option<PushedFilter> {
    let made = step_in(steps, id)?;
    let filtering = steps.get(reader.0)?;
    let Operation::Select {
        comparisons, kept, ..
    } = &filtering.operation
    else {
        return None;
    };
    let width = usize::try_from(width_of(steps, Input::Step(id))?.0).unwrap_or(usize::MAX);
    let prefix = fixed_prefix(comparisons, RowWidth(width)).0;
    if prefix.is_empty() {
        return None;
    }
    let first = StepId(u32::try_from(reader.0).unwrap_or(u32::MAX));
    let mut before = Vec::new();
    let filtered_left = place_after(first, &before);
    before.push(filter_of(steps, left, prefix.to_vec(), &filtering.written)?);
    let filtered_right = place_after(first, &before);
    before.push(filter_of(
        steps,
        right,
        prefix.to_vec(),
        &filtering.written,
    )?);
    let combined = PlanStep {
        operation: Operation::Union {
            left: filtered_left,
            right: filtered_right,
        },
        scope: made.scope,
        written: made.written.clone(),
    };
    let rest = comparisons.get(prefix.len()..).unwrap_or_default().to_vec();
    Some(finished(
        reader,
        before,
        combined,
        filtering,
        rest,
        kept.clone(),
    ))
}

//! Retained typed answers. Most slots are empty after screening.
use super::{Answer, Reported, worker::Slots};
use std::{ops::Index, sync::LazyLock};

const UNANSWERED: u32 = u32::MAX;
const EMPTY: u32 = u32::MAX - 1;
static UNANSWERED_ANSWER: Answer = Err(String::new());
static EMPTY_ANSWER: LazyLock<Answer> = LazyLock::new(|| Ok(Reported::default()));

/// Each admission slot holds a four-byte state or an index into the payloads.
/// Only answers with rows, partials, syntax skips or refusals own a payload.
#[derive(Default)]
pub(crate) struct ReportSlots {
    slots: Vec<u32>,
    payloads: Vec<Answer>,
    unused: Vec<u32>,
}

impl ReportSlots {
    /// The schedule's slot numbers remain valid in every later admission wave.
    pub(crate) fn grow(&mut self, len: usize) {
        if self.slots.len() < len {
            self.slots.resize(len, UNANSWERED);
        }
    }

    pub(crate) fn get(&self, slot: usize) -> Option<&Answer> {
        self.slots.get(slot).map(|&state| match state {
            UNANSWERED => &UNANSWERED_ANSWER,
            EMPTY => &*EMPTY_ANSWER,
            payload => &self.payloads[payload as usize],
        })
    }

    pub(crate) fn set(&mut self, slot: usize, answer: Answer) {
        let marker = match &answer {
            Err(reason) if reason.is_empty() => UNANSWERED,
            Ok(report)
                if report.rows.is_empty()
                    && report.rejected_syntax.is_empty()
                    && report.unknowns.is_empty() =>
            {
                EMPTY
            }
            _ => {
                let previous = self.slots[slot];
                if previous < EMPTY {
                    self.payloads[previous as usize] = answer;
                    return;
                }
                let payload = if let Some(payload) = self.unused.pop() {
                    self.payloads[payload as usize] = answer;
                    payload
                } else {
                    let payload = u32::try_from(self.payloads.len())
                        .ok()
                        .filter(|&id| id < EMPTY)
                        .expect("report payload index");
                    self.payloads.push(answer);
                    payload
                };
                self.slots[slot] = payload;
                return;
            }
        };
        self.clear(slot, marker);
    }

    /// The scheduler releases only after the last consumer and any store write.
    pub(crate) fn release(&mut self, slot: usize) {
        self.clear(slot, UNANSWERED);
    }

    fn clear(&mut self, slot: usize, marker: u32) {
        let previous = std::mem::replace(&mut self.slots[slot], marker);
        if previous < EMPTY {
            self.payloads[previous as usize] = Err(String::new());
            self.unused.push(previous);
        }
    }
}

impl Index<usize> for ReportSlots {
    type Output = Answer;

    fn index(&self, slot: usize) -> &Answer {
        self.get(slot).expect("admitted report slot")
    }
}

impl Slots<Answer> for ReportSlots {
    fn previous(&self, slot: usize) -> Option<&Answer> {
        self.get(slot)
    }

    fn set(&mut self, slot: usize, answer: Answer) {
        Self::set(self, slot, answer);
    }
}

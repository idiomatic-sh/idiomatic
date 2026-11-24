//! Keep the pair-to-answer index small. In a large capture most pairs are
//! aliases or have no rows, and neither needs its own owned answer.
use super::{Answer, Reported, worker::Slots};
use std::ops::Index;

const MISSING: u32 = 0;
const EMPTY: u32 = 1;
const FIRST_PAYLOAD: u32 = 2;

pub(crate) struct Answers {
    slots: Vec<u32>,
    payloads: Vec<Answer>,
    free: Vec<u32>,
    missing: Answer,
    empty: Answer,
}

impl Default for Answers {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            payloads: Vec::new(),
            free: Vec::new(),
            missing: Err(String::new()),
            empty: Ok(Reported::default()),
        }
    }
}

impl Answers {
    pub(crate) fn resize(&mut self, len: usize) {
        debug_assert!(len >= self.slots.len());
        self.slots.resize(len, MISSING);
    }
}

impl Index<usize> for Answers {
    type Output = Answer;

    fn index(&self, slot: usize) -> &Self::Output {
        match self.slots[slot] {
            MISSING => &self.missing,
            EMPTY => &self.empty,
            id => &self.payloads[(id - FIRST_PAYLOAD) as usize],
        }
    }
}

impl Slots<Answer> for Answers {
    fn previous(&self, slot: usize) -> Option<&Answer> {
        (slot < self.slots.len()).then(|| &self[slot])
    }

    fn set(&mut self, slot: usize, value: Answer) {
        let old = self.slots[slot];
        let tag = match &value {
            Ok(report) if report.rows.is_empty() && report.unknowns.is_empty() => EMPTY,
            Err(reason) if reason.is_empty() => MISSING,
            _ => {
                let into = if old >= FIRST_PAYLOAD {
                    old
                } else {
                    self.free.pop().unwrap_or_else(|| {
                        u32::try_from(self.payloads.len())
                            .ok()
                            .and_then(|n| n.checked_add(FIRST_PAYLOAD))
                            .expect("answer payload index fits in 32 bits")
                    })
                };
                let at = (into - FIRST_PAYLOAD) as usize;
                if at == self.payloads.len() {
                    self.payloads.push(value);
                } else {
                    self.payloads[at] = value;
                }
                self.slots[slot] = into;
                return;
            }
        };
        if old >= FIRST_PAYLOAD {
            self.payloads[(old - FIRST_PAYLOAD) as usize] = Err(String::new());
            self.free.push(old);
        }
        self.slots[slot] = tag;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_distinct_from_missing_and_partial() {
        let mut answers = Answers::default();
        answers.resize(3);
        answers.set(0, Ok(Reported::default()));
        answers.set(
            1,
            Ok(Reported {
                rows: Vec::new(),
                unknowns: vec![("path".into(), "partial".into())],
            }),
        );
        assert!(answers[0].is_ok());
        assert_eq!(answers[1].as_ref().unwrap().unknowns.len(), 1);
        assert_eq!(answers[2].as_ref().err().unwrap(), "");
        assert!(answers.previous(3).is_none());

        answers.set(1, Ok(Reported::default()));
        answers.resize(4);
        answers.set(3, Err("reader refused".into()));
        assert_eq!(answers.payloads.len(), 1);
        assert!(answers[1].is_ok());
        assert_eq!(answers[3].as_ref().err().unwrap(), "reader refused");
    }
}

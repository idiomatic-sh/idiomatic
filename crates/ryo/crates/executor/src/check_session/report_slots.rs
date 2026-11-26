//! Retained typed answers. Most slots are empty after screening.
use super::{Answer, Reported, worker::Slots};
use std::{ops::Index, sync::LazyLock};

pub const UNANSWERED: u32 = u32::MAX;
pub const EMPTY: u32 = u32::MAX - 1;
pub static UNANSWERED_ANSWER: Answer = Err(String::new());
pub static EMPTY_ANSWER: LazyLock<Answer> = LazyLock::new(|| Ok(Reported::default()));

/// An empty bit is set only by an explicit, complete empty report. A page
/// allocates its payload index on the first nonempty or refused answer.
#[derive(Default)]
pub struct Page {
    pub empty: u64,
    pub payload: Option<Box<[u32; 64]>>,
}

#[derive(Default)]
pub struct ReportSlots {
    pub pages: Vec<Page>,
    pub len: usize,
    pub payloads: Vec<Answer>,
    pub unused: Vec<u32>,
}

impl ReportSlots {
    /// The schedule's slot numbers remain valid in every later admission wave.
    pub fn grow(&mut self, len: usize) {
        if self.len < len {
            self.pages.resize_with(len.div_ceil(64), Page::default);
            self.len = len;
        }
    }

    pub fn get(&self, slot: usize) -> Option<&Answer> {
        if slot >= self.len { return None; }
        let page = &self.pages[slot / 64];
        let bit = slot % 64;
        if page.empty & (1 << bit) != 0 { return Some(&*EMPTY_ANSWER); }
        Some(match page.payload.as_ref().map_or(UNANSWERED, |ids| ids[bit]) {
            UNANSWERED => &UNANSWERED_ANSWER,
            id => &self.payloads[id as usize],
        })
    }

    /// Visit every slot except those explicitly established empty. An unanswered
    /// input still reaches the caller's refusal handling, including the last page.
    pub fn visit_pending<E>(
        &self, slots: std::ops::Range<usize>, mut visit: impl FnMut(usize) -> Result<(), E>,
    ) -> Result<(), E> {
        if slots.is_empty() { return Ok(()); }
        assert!(slots.end <= self.len);
        for page in slots.start / 64..= (slots.end - 1) / 64 {
            let base = page * 64;
            let lo = slots.start.saturating_sub(base);
            let hi = (slots.end - base).min(64);
            let mut pending = !self.pages[page].empty & (u64::MAX << lo) & (u64::MAX >> (64-hi));
            while pending != 0 {
                visit(base + pending.trailing_zeros() as usize)?;
                pending &= pending - 1;
            }
        }
        Ok(())
    }

    pub fn set(&mut self, slot: usize, answer: Answer) {
        let marker = match &answer {
            Err(reason) if reason.is_empty() => UNANSWERED,
            Ok(report)
                if report.rows.is_empty()
                    && report.rejected_syntax.is_empty()
                    && report.unknowns.is_empty()
                    && report.unread.is_empty() =>
            {
                EMPTY
            }
            _ => {
                let page = &mut self.pages[slot / 64];
                let offset = slot % 64;
                page.empty &= !(1 << offset);
                let ids = page.payload.get_or_insert_with(|| Box::new([UNANSWERED; 64]));
                let previous = ids[offset];
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
                ids[offset] = payload;
                return;
            }
        };
        self.clear(slot, marker);
    }

    /// The scheduler releases only after the last consumer and any store write.
    pub fn release(&mut self, slot: usize) {
        self.clear(slot, UNANSWERED);
    }

    pub fn clear(&mut self, slot: usize, marker: u32) {
        let page = &mut self.pages[slot / 64];
        let offset = slot % 64;
        if marker == EMPTY { page.empty |= 1 << offset; } else { page.empty &= !(1 << offset); }
        let Some(ids) = &mut page.payload else { return; };
        let previous = std::mem::replace(&mut ids[offset], UNANSWERED);
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

    fn set_empty(&mut self, slot: usize, _empty: impl FnOnce() -> Answer) {
        self.clear(slot, EMPTY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_skips_only_confirmed_empty_answers_within_the_range() {
        let mut slots = ReportSlots::default();
        slots.grow(160);
        for i in 60..132 { slots.set(i, Ok(Reported::default())); }
        slots.set(64, Err("partial".to_owned()));
        slots.release(130);
        let mut pending = Vec::new();
        slots.visit_pending::<()>(59..133, |i| { pending.push(i); Ok(()) }).unwrap();
        assert_eq!(pending, [59, 64, 130, 132]);
        assert_eq!(slots[64].as_ref().err().unwrap(), "partial");
        assert!(slots[130].is_err());
        assert!(slots[129].is_ok());
        slots.set(64, Ok(Reported::default()));
        assert!(slots[64].is_ok());
        assert!(slots.get(160).is_none());
    }
}

//! Borrowed capture contracts; no engine authority.
/// Engine-owned symbol collection for one complete path-capture match.
pub trait CaptureOutput {
    fn captures(&mut self, ranges: CaptureSpans<'_>);
}

/// The authored path pattern, with `{name}` and `{*name}` slots.
#[derive(Clone, Copy, Debug)]
pub struct SlotPattern<'a>(pub &'a str);

/// A path, or the part of it not yet matched.
#[derive(Clone, Copy, Debug)]
pub struct SlotSubject<'a>(pub &'a str);

/// The engine-owned ranges of the slots matched so far.
pub struct CaptureSlots<'a>(pub &'a mut Vec<std::ops::Range<usize>>);

/// The ranges of every slot of one complete match, in slot order.
#[derive(Clone, Copy, Debug)]
pub struct CaptureSpans<'a>(pub &'a [std::ops::Range<usize>]);

/// Interns what each slot of a complete match matched, and counts the matches.
pub(crate) struct SlotMinting<'a> {
    /// The engine's driver, which interns each matched slot.
    pub driver: &'a mut dyn crate::domain::contract::types::RowDriver,
    /// The text the pattern was matched against.
    pub text: &'a str,
    /// The interned slots of every complete match, in order.
    pub captured: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    /// How many complete matches there were.
    pub ways: usize,
}

impl crate::domain::capture::types::CaptureOutput for SlotMinting<'_> {
    fn captures(&mut self, ranges: crate::domain::capture::types::CaptureSpans<'_>) {
        for place in ranges.0 {
            let made = self
                .driver
                .mint(ryo_values::domain::row_texts::types::CellText(
                    self.text.get(place.clone()).unwrap_or_default(),
                ));
            self.captured.push(made);
        }
        self.ways += 1;
    }
}

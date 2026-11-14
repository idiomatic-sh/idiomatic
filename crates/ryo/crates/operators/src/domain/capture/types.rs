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
    pub text: SlotSubject<'a>,
    /// The interned slots of every complete match, in order.
    pub captured: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    /// How many complete matches there were.
    pub ways: ryo_relations::domain::relations::types::Count,
}

impl crate::domain::capture::types::CaptureOutput for SlotMinting<'_> {
    fn captures(&mut self, ranges: crate::domain::capture::types::CaptureSpans<'_>) {
        for place in ranges.0 {
            let made = self
                .driver
                .mint(ryo_values::domain::row_texts::types::CellText(
                    self.text.0.get(place.clone()).unwrap_or_default(),
                ));
            self.captured.push(made);
        }
        self.ways.0 += 1;
    }
}

/// What a capture lowers to: the path column and the slot pattern.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CaptureParts {
    /// The column holding the path.
    pub(crate) path: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The pattern, with its slots.
    pub(crate) pattern: ryo_values::domain::plan_parts::types::CapturePattern,
}

/// A capture's slot places, one way after another, kept to be minted once the match is done.
pub(crate) struct SlotPlaces<'a> {
    /// The place of every slot of every way so far, in way order.
    pub(crate) places: &'a mut crate::domain::contract::types::RangeScratch,
    /// How many ways the pattern matched.
    pub(crate) ways: ryo_relations::domain::relations::types::Count,
}

impl CaptureOutput for SlotPlaces<'_> {
    fn captures(&mut self, ranges: CaptureSpans<'_>) {
        self.places.0.extend_from_slice(ranges.0);
        self.ways.0 += 1;
    }
}

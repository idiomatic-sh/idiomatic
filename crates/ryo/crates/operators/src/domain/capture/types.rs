//! Borrowed capture contracts; no engine authority.
/// Engine-owned symbol collection for one complete path-capture match.
pub trait CaptureOutput {
    fn captures(&mut self, ranges: &[std::ops::Range<usize>]);
}

/// The authored path pattern, with `{name}` and `{*name}` slots.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SlotPattern<'a>(pub &'a str);

/// A path, or the part of it not yet matched.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SlotSubject<'a>(pub &'a str);

/// The engine-owned ranges of the slots matched so far.
pub(crate) struct CaptureSlots<'a>(pub &'a mut Vec<std::ops::Range<usize>>);

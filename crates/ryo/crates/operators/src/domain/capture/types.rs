//! Borrowed capture contracts; no engine authority.
/// Engine-owned symbol collection for one complete path-capture match.
pub trait CaptureOutput {
    fn captures(&mut self, ranges: &[std::ops::Range<usize>]);
}

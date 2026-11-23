//! Reset validity in one bit per register. The previous file's values are
//! inaccessible until a register has been written in this file.
#[derive(Default)]
pub(super) struct Memo<T> {
    valid: Vec<u64>,
    values: Vec<T>,
}
impl<T: Copy + Default> Memo<T> {
    pub(super) fn reset(&mut self, len: usize) {
        self.valid.resize(len.div_ceil(64), 0);
        self.valid.fill(0);
        self.values.resize(len, T::default());
    }
    #[inline]
    pub(super) fn get(&self, id: usize) -> Option<T> {
        (self.valid[id / 64] & (1 << (id % 64)) != 0).then(|| self.values[id])
    }
    #[inline]
    pub(super) fn set(&mut self, id: usize, value: T) {
        self.values[id] = value;
        self.valid[id / 64] |= 1 << (id % 64);
    }
}

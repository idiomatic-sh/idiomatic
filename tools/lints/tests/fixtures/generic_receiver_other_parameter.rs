pub trait LocalValue {}
pub mod interface {
    pub trait MeasureInterface<K> {
        fn measure(&self) -> foreign::Count;
    }
}
impl<T: Copy, K: LocalValue> interface::MeasureInterface<K> for T {
    fn measure(&self) -> foreign::Count { foreign::Count(1) }
}

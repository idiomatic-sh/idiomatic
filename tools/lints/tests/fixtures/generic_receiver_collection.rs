pub trait LocalValue: Sized {}
impl LocalValue for Vec<u32> {}
pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self) -> foreign::Count;
    }
}
impl<T: LocalValue> interface::MeasureInterface for T {
    fn measure(&self) -> foreign::Count { foreign::Count(1) }
}

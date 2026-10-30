pub trait LocalValue {}
pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self) -> foreign::Count;
    }
}
impl<T: LocalValue> interface::MeasureInterface for Option<T> {
    fn measure(&self) -> foreign::Count { foreign::Count(1) }
}

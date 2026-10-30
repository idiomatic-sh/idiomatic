pub trait LocalValue {}
pub trait RecursiveValue {}
impl<T: RecursiveValue> LocalValue for T {}
impl<T: LocalValue> RecursiveValue for T {}
pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self) -> foreign::Count;
    }
}
impl<T: LocalValue> interface::MeasureInterface for T {
    fn measure(&self) -> foreign::Count { foreign::Count(1) }
}

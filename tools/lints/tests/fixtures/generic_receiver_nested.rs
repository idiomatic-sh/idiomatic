pub trait LocalValue {}
pub trait NestedValue {}
pub struct Count;
impl NestedValue for Count {}
impl<T: NestedValue> LocalValue for T {}
pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self) -> foreign::Count;
    }
}
impl<T: LocalValue> interface::MeasureInterface for T {
    fn measure(&self) -> foreign::Count { foreign::Count(1) }
}

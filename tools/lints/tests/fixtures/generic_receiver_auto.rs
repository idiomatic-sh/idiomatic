#![feature(auto_traits)]
pub auto trait LocalValue {}
pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self) -> foreign::Count;
    }
}
impl<T: LocalValue> interface::MeasureInterface for T {
    fn measure(&self) -> foreign::Count { foreign::Count(1) }
}
pub fn primitive_accepted() -> foreign::Count {
    use interface::MeasureInterface;
    42_u32.measure()
}

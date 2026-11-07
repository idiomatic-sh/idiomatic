pub struct Measurement;
pub trait LocalValue: Sized {}
pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self) -> super::Measurement;
    }
}
impl<T: LocalValue> interface::MeasureInterface for T {
    fn measure(&self) -> Measurement { Measurement }
}

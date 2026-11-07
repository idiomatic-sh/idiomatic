pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self) -> foreign::Count;
    }
}
impl interface::MeasureInterface for u32 {
    fn measure(&self) -> foreign::Count {
        foreign::Count(*self)
    }
}

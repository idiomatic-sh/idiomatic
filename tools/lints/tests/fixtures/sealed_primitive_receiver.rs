mod sealed { pub trait Sealed {} impl Sealed for u32 {} }
pub mod interface {
    pub trait MeasureInterface: super::sealed::Sealed {
        fn measure(&self) -> foreign::Count;
    }
}
impl interface::MeasureInterface for u32 {
    fn measure(&self) -> foreign::Count { foreign::Count(1) }
}

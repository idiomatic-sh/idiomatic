pub mod interface {
    pub trait MeasureInterface {
        fn raw(&self) -> Result<foreign::Raw, foreign::Failure>;
    }
}

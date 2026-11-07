pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self, value: &[foreign::Raw]);
    }
}

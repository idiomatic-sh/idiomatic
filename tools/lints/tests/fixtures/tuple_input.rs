pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self, value: (foreign::Count, foreign::Raw));
    }
}

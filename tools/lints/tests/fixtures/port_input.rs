pub mod interface {
    pub trait MeasurePort {
        fn measure(&self, value: foreign::Raw);
    }
}

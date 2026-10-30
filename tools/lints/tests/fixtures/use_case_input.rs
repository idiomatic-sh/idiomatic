pub mod interface {
    pub trait MeasureUseCase {
        fn measure(&self, value: foreign::Raw);
    }
}

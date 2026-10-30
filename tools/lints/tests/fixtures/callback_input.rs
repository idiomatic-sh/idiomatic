pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self, value: fn(foreign::Raw) -> foreign::Count);
    }
}

pub mod interface {
    pub trait MeasureInterface {
        fn raw(&self) -> <foreign::Primitive as foreign::Carrier>::Value;
    }
}

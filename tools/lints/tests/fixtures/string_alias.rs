pub mod interface {
    pub trait MeasureInterface {
        fn raw(&self) -> foreign::Text;
    }
}

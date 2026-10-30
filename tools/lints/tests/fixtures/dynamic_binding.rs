pub mod interface {
    pub trait MeasureInterface {
        fn raw(&self) -> Box<dyn Iterator<Item = foreign::Raw>>;
    }
}

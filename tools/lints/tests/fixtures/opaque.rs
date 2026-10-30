pub mod interface {
    pub trait MeasureInterface {
        fn values(&self) -> impl Iterator<Item = foreign::Raw>;
    }
}

pub mod interface {
    pub trait MeasureInterface {
        fn measure(&self, name: &foreign::Name) -> Result<foreign::Count, foreign::Failure>;
        fn values(&self) -> Box<dyn Iterator<Item = foreign::Count>>;
    }
}

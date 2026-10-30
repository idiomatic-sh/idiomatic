pub trait External {
    fn raw(&self) -> u32;
}
pub struct Adapter;
impl External for Adapter {
    fn raw(&self) -> u32 {
        1
    }
}

pub mod interface {
    pub trait External {
        fn raw(&self) -> u32;
    }
    pub struct Adapter;
    impl External for Adapter {
        fn raw(&self) -> u32 { 1 }
    }
}
pub trait ExternalInterface {
    fn raw(&self) -> u32;
}
impl ExternalInterface for Adapter {
    fn raw(&self) -> u32 { 1 }
}

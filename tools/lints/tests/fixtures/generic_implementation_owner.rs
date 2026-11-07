pub struct Wrapper<T>(pub T);
pub mod interface {
    pub trait HoldInterface {
        type Held;
        fn held(&self) -> &Self::Held;
    }
}
impl<T> interface::HoldInterface for Wrapper<T> {
    type Held = T;
    fn held(&self) -> &T {
        &self.0
    }
}

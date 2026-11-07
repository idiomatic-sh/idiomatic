pub mod interface {
    pub trait SelectionInterface {
        type Choice;
        fn choose(&self) -> Self::Choice;
    }
}

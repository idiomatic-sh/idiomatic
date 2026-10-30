pub struct Adapter<T>(pub std::marker::PhantomData<T>);
impl<T> contract::interface::SelectionInterface for Adapter<T> {
    type Choice = foreign::Count;
    fn choose(&self) -> Self::Choice {
        foreign::Count(1)
    }
}

pub struct Adapter<T>(pub T);
impl<T: Copy> contract::interface::SelectionInterface for Adapter<T> {
    type Choice = T;
    fn choose(&self) -> Self::Choice {
        self.0
    }
}

use contract::interface::SelectionInterface as Selected;
pub struct Adapter;
impl Selected for Adapter {
    type Choice = foreign::Raw;
    fn choose(&self) -> Self::Choice {
        1
    }
}

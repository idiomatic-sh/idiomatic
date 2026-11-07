use contract::interface::SelectionInterface as Selected;
pub struct Adapter;
impl Selected for Adapter {
    type Choice = foreign::Amount;
    fn choose(&self) -> Self::Choice {
        foreign::Count(1)
    }
}

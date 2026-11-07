use contract::interface::MeasureInterface;
pub struct Distance;
impl contract::LocalValue for Distance {}
pub fn measure() -> contract::Measurement { Distance.measure() }

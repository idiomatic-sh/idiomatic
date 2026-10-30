pub struct Count(pub u32);
pub struct Name(pub String);
pub enum Failure {
    Invalid,
}
pub type Amount = Count;
pub type Raw = u32;
pub type Text = String;
pub type Values = Vec<Count>;
pub struct Primitive;
pub trait Carrier {
    type Value;
}
impl Carrier for Primitive {
    type Value = u32;
}

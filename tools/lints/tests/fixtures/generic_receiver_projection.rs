pub trait LocalValue { type Inner; }
pub struct Count;
impl LocalValue for Count { type Inner = u32; }
pub mod interface { pub trait MeasureInterface { type Output; } }
impl<T: LocalValue> interface::MeasureInterface for T { type Output = T::Inner; }

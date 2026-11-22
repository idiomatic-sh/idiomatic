//! The types of the `arguments` part: the number a step is named by. A relation an operator reads is a step of the
//! plan, by its number; every other argument is a column of an input, text, a number or a constant. Plain data.

/// A step's place in its plan's list of steps: the number every reader names it by. A step reads only steps before it.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::types::StepNumber;
///
/// assert!(StepNumber(1) < StepNumber(2));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepNumber(pub usize);

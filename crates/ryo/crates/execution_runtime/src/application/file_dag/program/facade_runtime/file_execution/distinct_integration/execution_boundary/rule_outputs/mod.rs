//! Rule outputs outlive workspace drops, and evaluation errors leave no mutable state behind.
#[cfg(test)]
mod test;

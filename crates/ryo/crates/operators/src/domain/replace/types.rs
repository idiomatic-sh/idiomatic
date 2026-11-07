//! The text a literal replacement is made of.
/// The authored text each match is replaced with.
#[derive(Clone, Copy, Debug)]
pub struct ReplacementText<'a>(pub &'a str);

/// The engine-owned scratch the replaced text is built in.
pub struct ReplacedText<'a>(pub &'a mut String);

//! The order a rule's refusals and row errors are answered in, around steps whose schema refuses, in each file and after the
//! files, observed or not: every answer the interpreter's at checkpoint-350 (64a99968a1), run on the same input and recorded.
#[cfg(test)]
mod fixtures;
mod frontiers;
mod generated;
#[cfg(test)]
mod test;
mod shapes;

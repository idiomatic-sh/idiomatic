//! The python domain's `layout_facts` concept: what the tokens of a walked file say of its layout: the links between its tokens,
//! its indentation regions and the lines its elements stand on.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;

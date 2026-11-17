//! The `closing` concept: which files a closed directory holds that nothing places, and which scope each is named after. The files and
//! what places them are facts tm hosts; which of them are strays and whose is each is a RyoQL query over those facts.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;

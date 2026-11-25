//! The request-level observation schedule: which rules read which producers' named outputs, the schemas their bound inputs
//! resolve to, and the plans linked beneath them. The request graph is built from it; it reads nothing of the graph.
pub mod fixtures;
pub mod service;
pub mod types;

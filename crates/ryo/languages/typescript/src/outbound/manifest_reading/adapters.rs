//! Repository project support: one adapter for each package manager, and one for the TypeScript project configuration.

pub(crate) mod bun;
pub(crate) mod npm;
pub(crate) mod pnpm;
pub(crate) mod tsconfig;

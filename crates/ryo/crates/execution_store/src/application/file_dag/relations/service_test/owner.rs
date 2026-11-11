//! Catalog-wide physical access and owner-only admission, independent of semantic completeness.
#[cfg(test)]
mod fixtures;
mod projection;
mod publication;
#[cfg(test)]
pub(super) mod test;

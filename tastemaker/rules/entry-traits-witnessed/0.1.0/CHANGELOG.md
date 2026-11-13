# entry-traits-witnessed 0.1.0

Every `pub trait` of an `interfaces` file is implemented by the `implementer` type of a `witness` file, or is a `{ trait, why }` row of `non-entries` with a reason. It replaces the submission census test `every_public_trait_is_an_entry_with_variants_or_is_named_with_its_reason`, which read the interface files as text and kept the non-entries in a `NON_ENTRY` constant. Fixtures `every-trait-is-witnessed-or-named` (pass), `a-new-trait-left-out` and `a-named-trait-without-a-reason` (violations).

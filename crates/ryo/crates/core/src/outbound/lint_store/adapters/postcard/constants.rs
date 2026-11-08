//! The names of the postcard store adapter: the text of the places it writes.

/// The file of the store in the cache directory.
pub(super) const FILE: &str = "lint.store";

/// What a file is called while it is written, before it is renamed into place.
pub(super) const TEMPORARY: &str = "lint.store.tmp";

/// What the store says when its place holds something that is not a file.
pub(super) const NOT_A_FILE: &str = "the store is not a file";

/// What the store says when its table of contents names a section that does not end inside the file.
pub(super) const OUTSIDE: &str = "the table of contents names a section past the end of the store";

/// What the store says when a section a run asks for does not read back.
pub(super) const UNDECODED: &str = "a section of the store does not read back";

/// How long the number that says how long the table of contents is, immediately before the checked format trailer.
pub(super) const LENGTH: u64 = 8;

/// Checked before positional postcard decoding; a store that does not carry it is a cache miss, and is rebuilt.
pub(super) const TRAILER: &[u8] =
    b"ryo-lint-store\0permissions\0observation-readiness\0project-entries\0project-path-kind\0frontier-proof\0proof-digests\0plan-identity\0";

/// The text "frontier payload lacks its exact source proof".
pub(super) const FRONTIER_PAYLOAD_LACKS_ITS_EXACT_SOURCE: &str =
    "frontier payload lacks its exact source proof";

/// The text "frontier proofs have duplicate or unordered source keys".
pub(super) const FRONTIER_PROOFS_HAVE_DUPLICATE_OR_UNORDERED: &str =
    "frontier proofs have duplicate or unordered source keys";

/// The text "no staging session was read".
pub(super) const NO_STAGING_SESSION_WAS_READ: &str = "no staging session was read";

/// The text "old bucket end overflows".
pub(super) const OLD_BUCKET_END_OVERFLOWS: &str = "old bucket end overflows";

/// The text "old row bucket or span is absent".
pub(super) const OLD_ROW_BUCKET_OR_SPAN_IS: &str = "old row bucket or span is absent";

/// The text "old row belongs to another store snapshot".
pub(super) const OLD_ROW_BELONGS_TO_ANOTHER_STORE: &str =
    "old row belongs to another store snapshot";

/// The text "store snapshot identities exhausted".
pub(super) const STORE_SNAPSHOT_IDENTITIES_EXHAUSTED: &str = "store snapshot identities exhausted";

/// The text "old row bucket is absent".
pub(super) const OLD_ROW_BUCKET_IS_ABSENT: &str = "old row bucket is absent";

/// The text "old bucket seek exceeds its section".
pub(super) const OLD_BUCKET_SEEK_EXCEEDS_ITS_SECTION: &str = "old bucket seek exceeds its section";

/// The text "old bucket logical position overflows".
pub(super) const OLD_BUCKET_LOGICAL_POSITION_OVERFLOWS: &str =
    "old bucket logical position overflows";

/// The text "old bucket position overflows".
pub(super) const OLD_BUCKET_POSITION_OVERFLOWS: &str = "old bucket position overflows";

/// The text "store offset overflows".
pub(super) const STORE_OFFSET_OVERFLOWS: &str = "store offset overflows";

/// The text "bucket size overflows".
pub(super) const BUCKET_SIZE_OVERFLOWS: &str = "bucket size overflows";

/// The text "no old cache snapshot was read".
pub(super) const NO_OLD_CACHE_SNAPSHOT_WAS_READ: &str = "no old cache snapshot was read";

/// The text "copied payload exceeds old snapshot".
pub(super) const COPIED_PAYLOAD_EXCEEDS_OLD_SNAPSHOT: &str = "copied payload exceeds old snapshot";

/// The text "payload exceeds postcard length width".
pub(super) const PAYLOAD_EXCEEDS_POSTCARD_LENGTH_WIDTH: &str =
    "payload exceeds postcard length width";

/// The text "inline size overflows".
pub(super) const INLINE_SIZE_OVERFLOWS: &str = "inline size overflows";

/// The text "header size overflows".
pub(super) const HEADER_SIZE_OVERFLOWS: &str = "header size overflows";

/// The text "old bucket varint is too long".
pub(super) const OLD_BUCKET_VARINT_IS_TOO_LONG: &str = "old bucket varint is too long";

/// The text "old bucket header overflows".
pub(super) const OLD_BUCKET_HEADER_OVERFLOWS: &str = "old bucket header overflows";

/// The text "old bucket header is truncated".
pub(super) const OLD_BUCKET_HEADER_IS_TRUNCATED: &str = "old bucket header is truncated";

/// The text "old row descriptor does not match its framing".
pub(super) const OLD_ROW_DESCRIPTOR_DOES_NOT_MATCH: &str =
    "old row descriptor does not match its framing";

/// The text "old row exceeds addressable memory".
pub(super) const OLD_ROW_EXCEEDS_ADDRESSABLE_MEMORY: &str = "old row exceeds addressable memory";

/// The text "old row header exceeds its encoded field widths".
pub(super) const OLD_ROW_HEADER_EXCEEDS_ITS_ENCODED: &str =
    "old row header exceeds its encoded field widths";

/// The text "old row header exceeds addressable memory".
pub(super) const OLD_ROW_HEADER_EXCEEDS_ADDRESSABLE_MEMORY: &str =
    "old row header exceeds addressable memory";

/// The text "old row header is inverted".
pub(super) const OLD_ROW_HEADER_IS_INVERTED: &str = "old row header is inverted";

/// The text "old row payload exceeds addressable memory".
pub(super) const OLD_ROW_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY: &str =
    "old row payload exceeds addressable memory";

/// The text "old row header is outside its bucket".
pub(super) const OLD_ROW_HEADER_IS_OUTSIDE_ITS: &str = "old row header is outside its bucket";

/// The text "old row payload is truncated or overflows".
pub(super) const OLD_ROW_PAYLOAD_IS_TRUNCATED_OR: &str =
    "old row payload is truncated or overflows";

/// The text "old bucket span is truncated or overflows".
pub(super) const OLD_BUCKET_SPAN_IS_TRUNCATED_OR: &str =
    "old bucket span is truncated or overflows";

/// The text "old bucket has trailing bytes".
pub(super) const OLD_BUCKET_HAS_TRAILING_BYTES: &str = "old bucket has trailing bytes";

/// The text "old file number overflows".
pub(super) const OLD_FILE_NUMBER_OVERFLOWS: &str = "old file number overflows";

/// The text "old row payload exceeds its section".
pub(super) const OLD_ROW_PAYLOAD_EXCEEDS_ITS_SECTION: &str = "old row payload exceeds its section";

/// The text "old row count exceeds its section".
pub(super) const OLD_ROW_COUNT_EXCEEDS_ITS_SECTION: &str = "old row count exceeds its section";

/// The text "missing inline fresh row".
pub(super) const MISSING_INLINE_FRESH_ROW: &str = "missing inline fresh row";

/// The text "missing staged row".
pub(super) const MISSING_STAGED_ROW: &str = "missing staged row";

/// The text "fresh row count exceeds staging width".
pub(super) const FRESH_ROW_COUNT_EXCEEDS_STAGING_WIDTH: &str =
    "fresh row count exceeds staging width";

/// The text "invalid fresh staged row order".
pub(super) const INVALID_FRESH_STAGED_ROW_ORDER: &str = "invalid fresh staged row order";

/// The text "fresh row count overflows".
pub(super) const FRESH_ROW_COUNT_OVERFLOWS: &str = "fresh row count overflows";

/// The text "old bucket seek underflows".
pub(super) const OLD_BUCKET_SEEK_UNDERFLOWS: &str = "old bucket seek underflows";

/// The text "old row seek underflows".
pub(super) const OLD_ROW_SEEK_UNDERFLOWS: &str = "old row seek underflows";

/// The text "old row position overflows".
pub(super) const OLD_ROW_POSITION_OVERFLOWS: &str = "old row position overflows";

/// The text "copy size overflows".
pub(super) const COPY_SIZE_OVERFLOWS: &str = "copy size overflows";

/// The text "copied payload is truncated".
pub(super) const COPIED_PAYLOAD_IS_TRUNCATED: &str = "copied payload is truncated";

/// The text "staged payload exceeds addressable memory".
pub(super) const STAGED_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY: &str =
    "staged payload exceeds addressable memory";

/// The text "staged offset overflows".
pub(super) const STAGED_OFFSET_OVERFLOWS: &str = "staged offset overflows";

/// The text "staged payload length overflows".
pub(super) const STAGED_PAYLOAD_LENGTH_OVERFLOWS: &str = "staged payload length overflows";

/// The text "staged span is outside this snapshot".
pub(super) const STAGED_SPAN_IS_OUTSIDE_THIS_SNAPSHOT: &str =
    "staged span is outside this snapshot";

/// The text "foreign or stale staging session".
pub(super) const FOREIGN_OR_STALE_STAGING_SESSION: &str = "foreign or stale staging session";

/// The text "missing spool reader".
pub(super) const MISSING_SPOOL_READER: &str = "missing spool reader";

/// The text "missing spool writer".
pub(super) const MISSING_SPOOL_WRITER: &str = "missing spool writer";

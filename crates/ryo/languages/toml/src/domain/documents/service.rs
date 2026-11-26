//! The `documents` service: it works out, in one pass over a file's syntax tree, the mappings, entries and sequence elements its TOML
//! declares, and answers each reader from that pass. A key is an entry where it first opens its mapping, or where it is the last
//! part of a key-value; a key a later header or dotted key says again is the same entry, and writes none. Every logical table has
//! one identity, whether a header, a dotted key or a longer header opens it, and a path through an array of tables resolves within
//! its most recent member, as TOML says. An ordinal counts among the entries of the owner the entry resolves to.
//! The pass is made once for a file, as its `Parse::FileModel`, which the engine gives every reader of the file.


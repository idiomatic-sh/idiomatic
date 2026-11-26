//! The library's own tree of a file, walked in the language's kinds: what the shape rail of the conformance holds our tree to.
//! `toml_parser` gives no tree, only a stream of events (`events`), every byte of the file in one of them, so its tree is read from
//! them in passes that share nothing with the adapter's one. First the constructs of TOML the events make, each with the bytes it
//! spans: a table from its header to the next one, a key-value from its key to the end of its value (or, with none, to the last piece
//! before what ends it), a key over its parts and the dots between them, an array or an inline table from its opening to its
//! closing. Then every event is a token, put in the deepest construct whose bytes hold it, an empty one, which has no bytes, in the
//! construct the parser gave it in. A token the adapter's pass puts in the wrong construct, or a construct it opens or closes in the
//! wrong place, is a difference.


















#[cfg(test)]
#[path = "walk_test.rs"]
mod tests;

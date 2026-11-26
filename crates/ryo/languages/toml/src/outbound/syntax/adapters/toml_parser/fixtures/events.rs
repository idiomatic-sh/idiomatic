//! The library's own events of a file, as the parser gives them: what the walk reads the library's tree from. `toml_parser` gives a
//! stream of events, every byte of the file in one of them; each is recorded with what it is to the constructs of TOML and the kind of
//! its token, the scalars as the library's decoder classifies them. Arrays and inline tables nest no deeper than the `toml` crate
//! reads, as the adapter tells the parser.




















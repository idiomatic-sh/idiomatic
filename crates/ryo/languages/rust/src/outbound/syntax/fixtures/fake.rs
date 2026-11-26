//! The fake of the `syntax` port: what the language's tests stand in for it with. It replays a tree written by hand (`script.rs`) and
//! reads nothing of the text it is given, so the text of a request must be the text the tree spells. It reads no macro again, and the value of
//! a plain string token (no escape) is the text between its quotes.








#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;

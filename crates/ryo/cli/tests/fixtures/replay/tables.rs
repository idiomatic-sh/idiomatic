//! Reads the TOML tables the replay tests look into, so a test names none of the library: each function takes the text of a
//! table and the keys that lead to what it wants.

use toml::Table;

/// The value at `keys`, a key for each level of table above it.
fn at<'a>(table: &'a Table, keys: &[&str]) -> Option<&'a toml::Value> {
    let (last, parents) = keys.split_last()?;
    let mut current = table;
    for key in parents {
        current = current.get(*key)?.as_table()?;
    }
    current.get(*last)
}

/// The table the text holds.
fn table(text: &str) -> Table {
    text.parse().expect("a table that reads")
}

/// The string at `keys`, when there is one.
pub(crate) fn string_under(text: &str, keys: &[&str]) -> Option<String> {
    at(&table(text), keys)?.as_str().map(str::to_owned)
}

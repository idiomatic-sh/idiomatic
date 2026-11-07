pub fn normalise_title(title: &str) -> String {
    let trimmed = title.trim();
    let single = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    single.to_uppercase()
}

// planted: Complexity limits
pub fn describe(title: &str, done: bool, owner: &str, due: &str, tag: &str) -> String {
    [title, if done { "done" } else { "open" }, owner, due, tag].join(" ")
}

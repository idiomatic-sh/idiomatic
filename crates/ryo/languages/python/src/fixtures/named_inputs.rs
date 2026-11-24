//! Type-expression declarations supplied by the calling rule pack.
use std::collections::BTreeSet;

/// Qualified generic heads, argument forms and their module prefixes.
#[derive(Default, Debug)]
pub struct Lists {
    /// Subscript heads with type arguments.
    pub generic: BTreeSet<Vec<u8>>,
    /// Call forms with a type argument.
    pub argument_forms: BTreeSet<Vec<u8>>,
    /// Dotted prefixes shared by the two sets.
    pub modules: Vec<Vec<u8>>,
}

impl Lists {
    /// Parse the pack's tab-separated declarations.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lists = Self::default();
        let mut modules = BTreeSet::new();
        for line in text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let (kind, path) = line
                .split_once('\t')
                .ok_or_else(|| format!("invalid type-expression declaration {line}"))?;
            if path
                .split('.')
                .any(|word| word.is_empty() || word.chars().any(char::is_whitespace))
            {
                return Err(format!("invalid type-expression name {path}"));
            }
            let names = match kind {
                "generic" => &mut lists.generic,
                "argument-form" => &mut lists.argument_forms,
                _ => return Err(format!("unknown type-expression declaration {kind}")),
            };
            names.insert(path.as_bytes().to_vec());
            modules.insert(format!("{}.", path.split('.').next().unwrap_or(path)).into_bytes());
        }
        lists.modules = modules.into_iter().collect();
        Ok(lists)
    }

    /// Load the optional declarations from the pack at its composition edge.
    pub fn load(pack: &std::path::Path) -> Result<Self, String> {
        let path = pack.join("inputs/type-expression.tsv");
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }
}

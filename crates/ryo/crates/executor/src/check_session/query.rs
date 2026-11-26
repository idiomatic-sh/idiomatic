//! Source identity for a prepared program, including the imports supplied with its root.
use ryo_query_language::outbound::syntax::{factory::winnow, interface::SyntaxPort};
use ryo_values::domain::{
    program::types::ParsedModules,
    query_tokens::types::{
        QuerySourceId, QueryView,
    },
};
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Modules {
    pub sources: Arc<BTreeMap<Box<str>, Arc<str>>>,
    pub digest: [u8; 32],
}

impl Modules {
    pub fn new(sources: BTreeMap<Box<str>, Arc<str>>) -> Self {
        let mut digest = blake3::Hasher::new();
        for (name, text) in &sources {
            for value in [name.as_bytes(), text.as_bytes()] {
                digest.update(&(value.len() as u64).to_le_bytes());
                digest.update(value);
            }
        }
        Self { sources: Arc::new(sources), digest: *digest.finalize().as_bytes() }
    }
}

impl Hash for Modules {
    fn hash<H: Hasher>(&self, state: &mut H) { self.digest.hash(state); }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Query {
    pub observations: Rc<[crate::lower::Observation]>,
    pub accepted_only: bool,
    /// Every row the query emits is kept, in the order the plan produces them: no two rows are one for sharing a rule, subject and key.
    pub every_row: bool,
    pub text: Rc<str>,
    /// The text's content digest. Identity and hashing use it, so a large root is never hashed twice.
    pub digest: [u8; 32],
    pub modules: Option<Modules>,
}

impl From<Rc<str>> for Query {
    fn from(text: Rc<str>) -> Self {
        Self {
            digest: *blake3::hash(text.as_bytes()).as_bytes(),
            text,
            modules: None,
            observations: Rc::default(),
            accepted_only: false,
            every_row: false,
        }
    }
}

impl Hash for Query {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.digest.hash(state);
        if !self.observations.is_empty() {
            self.observations.hash(state);
        }
        if self.accepted_only {
            true.hash(state);
        }
        if self.every_row {
            [true, true].hash(state);
        }
        if let Some(modules) = &self.modules {
            modules.hash(state);
        }
    }
}

impl Query {
    pub fn parse_modules(&self) -> Result<ParsedModules, String> {
        let Some(sources) = &self.modules else {
            return Ok(ParsedModules::default());
        };
        let syntax = winnow();
        let mut parsed = BTreeMap::new();
        for (name, text) in sources.sources.iter() {
            let id = QuerySourceId(Arc::from(name.as_ref()));
            let fragment = syntax
                .read_fragment(&id, QueryView(text))
                .map_err(|e| format!("module {name}: {e}"))?;
            parsed.insert(id, fragment);
        }
        Ok(ParsedModules(Arc::new(parsed)))
    }
}

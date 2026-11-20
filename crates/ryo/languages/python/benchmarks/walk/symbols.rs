//! Interned texts: the pack's literals, kind and field names are base symbols (stable ids, interned at load and when the view
//! first names a kind); a file's own texts get ids from FILE_BASE up, stored once in a reusable arena, reset per file.
//! Equal text = equal id.

use crate::sema::{FxMap, hash_text};

pub(crate) const NULL: u32 = u32::MAX;
const FILE_BASE: u32 = 1 << 30;

#[derive(Clone, Default)]
pub(crate) struct Symbols {
    base_map: FxMap<String, u32>,
    base_text: Vec<String>,
    /// hash (probed on collision) -> file symbol
    file_map: FxMap<u64, u32>,
    arena: String,
    spans: Vec<(u32, u32)>,
}

impl Symbols {
    pub(crate) fn base(&mut self, text: &str) -> u32 {
        if let Some(&id) = self.base_map.get(text) {
            return id;
        }
        let id = self.base_text.len() as u32;
        self.base_text.push(text.to_owned());
        self.base_map.insert(text.to_owned(), id);
        id
    }

    pub(crate) fn base_count(&self) -> usize {
        self.base_text.len()
    }

    /// exec: a text code as a dense index (base symbols, then the file's symbols in the order they were met), and the
    /// number of such indexes; None for a code that is neither (the null text)
    pub(crate) fn dense(&self, id: u32) -> Option<usize> {
        if id == NULL {
            None
        } else if id >= FILE_BASE {
            let k = (id - FILE_BASE) as usize;
            (k < self.spans.len()).then(|| self.base_text.len() + k)
        } else {
            ((id as usize) < self.base_text.len()).then_some(id as usize)
        }
    }

    pub(crate) fn dense_count(&self) -> usize {
        self.base_text.len() + self.spans.len()
    }

    /// every base symbol, in id order (the compiled pack's constant pool)
    pub(crate) fn base_texts(&self) -> &[String] {
        &self.base_text
    }

    pub(crate) fn get_base(&self, text: &str) -> Option<u32> {
        self.base_map.get(text).copied()
    }

    pub(crate) fn reset_file(&mut self) {
        self.file_map.clear();
        self.arena.clear();
        self.spans.clear();
    }

    fn file_str(&self, index: usize) -> &str {
        let (s, e) = self.spans[index];
        &self.arena[s as usize..e as usize]
    }

    /// A text of the current file.
    pub(crate) fn text(&mut self, text: &str) -> u32 {
        if let Some(&id) = self.base_map.get(text) {
            return id;
        }
        let mut h = hash_text(text);
        loop {
            match self.file_map.get(&h) {
                Some(&id) if self.file_str((id - FILE_BASE) as usize) == text => return id,
                Some(_) => h = h.wrapping_add(0x9e37_79b9_7f4a_7c15),
                None => break,
            }
        }
        let id = FILE_BASE + self.spans.len() as u32;
        let s = self.arena.len() as u32;
        self.arena.push_str(text);
        self.spans.push((s, self.arena.len() as u32));
        self.file_map.insert(h, id);
        id
    }

    /// A text minted during evaluation (a capture, a replacement): interned as a file text.
    pub(crate) fn minted(&mut self, text: String) -> u32 {
        self.text(&text)
    }

    pub(crate) fn str(&self, id: u32) -> Option<&str> {
        if id == NULL {
            None
        } else if id >= FILE_BASE {
            let index = (id - FILE_BASE) as usize;
            (index < self.spans.len()).then(|| self.file_str(index))
        } else {
            self.base_text.get(id as usize).map(String::as_str)
        }
    }
}

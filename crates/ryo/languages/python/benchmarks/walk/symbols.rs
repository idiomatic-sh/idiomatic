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
    /// facts: a word fixed in the provider's code (by address) -> its base symbol; words first met as a file text become base
    /// symbols at the next file boundary (never inside a file: the file may already hold the text under a file id)
    word_map: FxMap<(usize, usize), u32>,
    pending: Vec<&'static str>,
    /// the last words looked up, direct-mapped by address: (address, length, code)
    word_last: Vec<(usize, usize, u32)>,
    /// facts: texts interned lately, direct-mapped by (length, first and last eight bytes): (signature, code, file number);
    /// a hit is checked against the stored text, so a repeated text costs one comparison and no hashing
    front: Vec<(u64, u32, u32)>,
    file_no: u32,
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
        self.file_no = self.file_no.wrapping_add(1);
        while let Some(w) = self.pending.pop() {
            let id = self.base(w);
            self.word_map.insert((w.as_ptr() as usize, w.len()), id);
        }
    }

    /// A word fixed in the provider's code (a kind, an outcome, a label): its code without hashing the text (facts).
    #[inline]
    pub(crate) fn word(&mut self, w: &'static str) -> u32 {
        let (p, n) = (w.as_ptr() as usize, w.len());
        let slot = (p >> 2) & 255;
        if let Some(&(lp, ln, code)) = self.word_last.get(slot) {
            if lp == p && ln == n {
                return code;
            }
        }
        if let Some(&id) = self.word_map.get(&(p, n)) {
            if self.word_last.is_empty() {
                self.word_last.resize(256, (0, 0, 0));
            }
            self.word_last[slot] = (p, n, id);
            return id;
        }
        self.word_slow(w)
    }

    #[inline(never)]
    fn word_slow(&mut self, w: &'static str) -> u32 {
        let id = self.text(w);
        if id < FILE_BASE {
            self.word_map.insert((w.as_ptr() as usize, w.len()), id);
        } else {
            self.pending.push(w);
        }
        id
    }

    /// A kind or field name fixed in the provider's code: a base symbol from its first use, found by address afterwards.
    #[inline]
    pub(crate) fn base_word(&mut self, w: &'static str) -> u32 {
        let key = (w.as_ptr() as usize, w.len());
        if let Some(&id) = self.word_map.get(&key) {
            return id;
        }
        let id = self.base(w);
        self.word_map.insert(key, id);
        id
    }

    pub(crate) fn is_base(id: u32) -> bool {
        id < FILE_BASE
    }

    fn file_str(&self, index: usize) -> &str {
        let (s, e) = self.spans[index];
        &self.arena[s as usize..e as usize]
    }

    /// A text of the current file.
    #[inline]
    pub(crate) fn text(&mut self, text: &str) -> u32 {
        let b = text.as_bytes();
        let n = b.len();
        // the signature: length, first eight bytes, last eight bytes (short texts: the bytes themselves)
        let (head, tail) = if n >= 8 {
            (u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]), u64::from_le_bytes([b[n - 8], b[n - 7], b[n - 6], b[n - 5], b[n - 4], b[n - 3], b[n - 2], b[n - 1]]))
        } else {
            let mut w = [0u8; 8];
            w[..n].copy_from_slice(b);
            (u64::from_le_bytes(w), 0)
        };
        let sig = (head ^ tail.rotate_left(29) ^ (n as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)).wrapping_mul(0xff51_afd7_ed55_8ccd);
        let slot = (sig >> 52) as usize;
        if self.front.is_empty() {
            self.front.resize(4096, (0, NULL, 0));
        }
        let (fs, code, file) = self.front[slot];
        if fs == sig && code != NULL && (code < FILE_BASE || file == self.file_no) && self.str(code) == Some(text) {
            return code;
        }
        let code = self.text_slow(text);
        self.front[slot] = (sig, code, self.file_no);
        code
    }

    fn text_slow(&mut self, text: &str) -> u32 {
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

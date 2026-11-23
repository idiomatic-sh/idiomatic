//! A provider can own the primary token ranges while the node table owns id mapping.
//! Synthetic tokens or a parser-adjusted range retain their explicit byte span.
use super::types::{ByteSpan, NodeTableEngine};
use ryo_relations::domain::relations::types::Offset;
use std::sync::OnceLock;

const EXPLICIT: u32 = 1 << 31;

/// Byte ranges in a provider-owned primary parse.
pub trait TokenSpanSource: std::fmt::Debug + Send + Sync {
    /// Source range of a token the provider registered by this key.
    fn token_span(&self, key: u32) -> ByteSpan;
}

#[derive(Debug, Default)]
pub(super) struct NativeSpans {
    pub(super) requested: bool,
    pub(super) source: Option<Box<dyn TokenSpanSource>>,
    pub(super) codes: Vec<u32>,
    explicit: Vec<ByteSpan>,
    first: usize,
    count: usize,
    pub(super) starts: OnceLock<Vec<u32>>,
    pub(super) ends: OnceLock<Vec<u32>>,
}
impl NativeSpans {
    pub(super) fn reset(&mut self) {
        self.requested = false;
        self.source = None;
        self.codes.clear();
        self.explicit.clear();
        self.first = 0;
        self.count = 0;
        self.invalidate();
    }
    pub(super) fn invalidate(&mut self) {
        drop(self.starts.take());
        drop(self.ends.take());
    }
    pub(super) fn register(&mut self, index: usize, code: u32) {
        if self.source.is_some() && code < EXPLICIT {
            self.codes.resize(index + 1, u32::MAX);
            self.codes[index] = code;
        }
    }
    pub(super) fn close(&mut self, first: usize, tokens: &[super::types::PendingToken]) -> bool {
        if self.source.is_none() || tokens.len() >= EXPLICIT as usize {
            self.source = None;
            return false;
        }
        self.first = first;
        self.count = tokens.len();
        self.codes.resize(tokens.len(), u32::MAX);
        for (code, t) in self.codes.iter_mut().zip(tokens) {
            if *code == u32::MAX {
                *code = EXPLICIT | self.explicit.len() as u32;
                self.explicit.push(t.span);
            }
        }
        true
    }
    pub(super) fn stored(&self, i: usize) -> usize {
        if i >= self.first + self.count {
            i - self.count
        } else {
            i
        }
    }
    pub(super) fn span(&self, i: usize) -> Option<ByteSpan> {
        if i < self.first || i >= self.first + self.count {
            return None;
        }
        let key = self.codes[i - self.first];
        if key & EXPLICIT != 0 {
            self.explicit.get((key & !EXPLICIT) as usize).copied()
        } else {
            self.source.as_ref().map(|s| s.token_span(key))
        }
    }
    pub(super) fn active(&self) -> bool {
        self.count != 0
    }
}
pub(super) fn span_at(columns: &NodeTableEngine, i: usize) -> ByteSpan {
    if let Some(s) = columns.native.span(i) {
        return s;
    }
    let i = columns.native.stored(i);
    match (columns.start.get(i), columns.end.get(i)) {
        (Some(&a), Some(&b)) => ByteSpan {
            start: Offset(a),
            end: Offset(b),
        },
        _ => ByteSpan::default(),
    }
}

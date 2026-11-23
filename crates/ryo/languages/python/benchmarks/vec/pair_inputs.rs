//! One native kind-bucket scan supplies the kind mask, literal postings and
//! decoded texts of regex subjects. Repeated lexemes share their text tests.
use super::*;
pub(super) struct BucketText<'a> {
    mask: Bits,
    values: Map<&'a [u8], Vec<u32>>,
}
pub(crate) struct Inputs {
    grouped: Vec<bool>,
}
pub(crate) fn prepare(p: &Program) -> Inputs {
    let mut grouped = Vec::new();
    for k in p
        .text_kinds
        .iter()
        .copied()
        .chain(p.domains.iter().filter_map(|d| {
            if let Domain::TextMatching(k, _) = d {
                Some(*k)
            } else {
                None
            }
        }))
    {
        if grouped.len() <= k as usize {
            grouped.resize(k as usize + 1, false);
        }
        grouped[k as usize] = true;
    }
    Inputs { grouped }
}
impl<'a> File<'a> {
    fn bucket_text(&mut self, k: u16) {
        if self.bucket_texts.contains_key(&k) {
            return;
        }
        let t = self.t;
        let bytes = self.src;
        let ids = node_table::ids_of_kind(t, languages::KindBits(k));
        let mut values = Map::<&'a [u8], Vec<u32>>::default();
        for &at in ids {
            let text = node_table::decoded_of(t, at).map_or_else(
                || &bytes[super::super::span_bytes(t, at.0 as usize)],
                |v| v.0.as_bytes(),
            );
            values.entry(text).or_default().push(at.0);
            insert(&mut self.arena.work, at.0);
        }
        let mask = self.arena.finish();
        self.bucket_texts.insert(k, BucketText { mask, values });
        self.bucket_requests += 1;
    }
    pub(super) fn literal_postings(&mut self) -> bool {
        if self.order.and_then(|o| o.inputs.as_ref()).is_none() {
            return false;
        }
        if self.texts.is_some() {
            return true;
        }
        let mut map = Map::<&'a [u8], Vec<u32>>::default();
        for &k in &self.p.text_kinds {
            self.bucket_text(k);
            for (&value, ids) in &self.bucket_texts[&k].values {
                if value.len() <= self.p.max_text && self.p.texts.contains(value) {
                    map.entry(value).or_default().extend_from_slice(ids);
                }
            }
        }
        self.texts = Some(map);
        true
    }
    pub(super) fn input_mask(&mut self, id: Id) -> Option<Nodes> {
        let inputs = self.order?.inputs.as_ref()?;
        let (k, re) = match self.p.domains[id] {
            Domain::Kind(k) => (k, None),
            Domain::TextMatching(k, re) => (k, Some(re)),
            _ => return None,
        };
        if !inputs.grouped.get(k as usize).copied().unwrap_or(false) {
            return None;
        }
        self.bucket_text(k);
        let bucket = &self.bucket_texts[&k];
        let mask = if let Some(re) = re {
            let regex = &self.p.regexes[re];
            self.arena.bits(
                bucket
                    .values
                    .iter()
                    .filter(|(value, _)| regex.is_match(value))
                    .flat_map(|(_, ids)| ids.iter().copied()),
            )
        } else {
            bucket.mask
        };
        let value = Some(mask);
        self.sets[id] = Some(value);
        Some(value)
    }
}

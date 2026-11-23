use std::collections::HashMap;

/// The file skip, flat: each rule's kinds-and-fields clauses and its word CNFs as numbers of distinct clauses, a clause
/// stored once however many rules share it. A file's kinds-and-fields clauses are all tested once (`file`); a word clause
/// is tested once a file, when a rule its kinds keep first asks it.
#[derive(Clone)]
pub(super) struct Skip {
    kw: usize,
    fw: usize,
    /// The distinct kinds-and-fields clauses, `kw + fw` words each: kind words then field words.
    masks: Vec<u64>,
    /// Per rule: never matches, and its clauses (a range of `of`).
    rules: Vec<(bool, std::ops::Range<u32>)>,
    of: Vec<u32>,
    /// This file's answer of each distinct kinds-and-fields clause.
    held: Vec<bool>,
    /// Per rule its branches (a range of `branches`; none: unknown, alive), a branch its clauses (a range of `wof`), a
    /// distinct word clause its words (a range of `wwords`).
    wrules: Vec<std::ops::Range<u32>>,
    branches: Vec<std::ops::Range<u32>>,
    wof: Vec<u32>,
    wclauses: Vec<std::ops::Range<u32>>,
    wwords: Vec<u16>,
    /// Per distinct word clause: `stamp << 1 | answer` once asked in the file of `stamp`.
    wmemo: Vec<u32>,
    stamp: u32,
}

impl Skip {
    pub(super) fn new<'a>(
        rules: impl Iterator<Item = (bool, &'a [(Vec<u64>, Vec<u64>)])>,
        word_cnfs: impl Iterator<Item = &'a [Vec<Vec<u16>>]>,
        kw: usize,
        fw: usize,
    ) -> Self {
        let mut masks: Vec<u64> = Vec::new();
        let mut seen: HashMap<Vec<u64>, u32> = HashMap::new();
        let mut of = Vec::new();
        let rules: Vec<(bool, std::ops::Range<u32>)> = rules
            .map(|(never, clauses)| {
                let a = of.len() as u32;
                for (k, f) in clauses {
                    let c: Vec<u64> = k.iter().chain(f).copied().collect();
                    assert_eq!(c.len(), kw + fw, "a clause is kw + fw words");
                    let n = seen.len() as u32;
                    let id = *seen.entry(c.clone()).or_insert_with(|| {
                        masks.extend_from_slice(&c);
                        n
                    });
                    of.push(id);
                }
                (never, a..of.len() as u32)
            })
            .collect();
        let mut wseen: HashMap<Vec<u16>, u32> = HashMap::new();
        let (mut wrules, mut branches, mut wof, mut wclauses, mut wwords) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for branches_of_rule in word_cnfs {
            let a = branches.len() as u32;
            for cnf in branches_of_rule {
                let b = wof.len() as u32;
                for cl in cnf {
                    let n = wseen.len() as u32;
                    let id = *wseen.entry(cl.clone()).or_insert_with(|| {
                        let w = wwords.len() as u32;
                        wwords.extend_from_slice(cl);
                        wclauses.push(w..wwords.len() as u32);
                        n
                    });
                    wof.push(id);
                }
                branches.push(b..wof.len() as u32);
            }
            wrules.push(a..branches.len() as u32);
        }
        let n_masks = masks.len() / (kw + fw).max(1);
        let n_w = wclauses.len();
        Skip {
            kw,
            fw,
            masks,
            rules,
            of,
            held: vec![false; n_masks],
            wrules,
            branches,
            wof,
            wclauses,
            wwords,
            wmemo: vec![0; n_w],
            stamp: 0,
        }
    }

    /// A new file: every distinct kinds-and-fields clause tested once against its kinds and fields; the word answers of
    /// the last file forgotten.
    pub(super) fn file(&mut self, kinds: &[u64], fields: &[u64]) {
        let (kw, fw) = (self.kw, self.fw);
        for (c, h) in self.masks.chunks_exact(kw + fw).zip(&mut self.held) {
            *h = c[..kw].iter().zip(kinds).any(|(a, b)| a & b != 0)
                || c[kw..].iter().zip(fields).any(|(a, b)| a & b != 0);
        }
        self.stamp += 1;
    }

    /// Whether every clause of the rule meets a kind or a field the file has (`survives` over the distinct clauses).
    #[inline]
    pub(super) fn alive(&self, rule: usize) -> bool {
        let (never, range) = &self.rules[rule];
        !never
            && self.of[range.start as usize..range.end as usize]
                .iter()
                .all(|c| self.held[*c as usize])
    }

    /// Whether some branch of the rule's word CNFs has every clause met by a word the file has (`FRule::words_alive`).
    #[inline]
    pub(super) fn words_alive(&mut self, rule: usize, present: &[u64]) -> bool {
        let r = self.wrules[rule].clone();
        if r.is_empty() {
            return true;
        }
        let stamp = self.stamp << 1;
        for b in r {
            let br = self.branches[b as usize].clone();
            let mut all = true;
            for k in br {
                let c = self.wof[k as usize] as usize;
                let m = self.wmemo[c];
                let held = if m & !1 == stamp {
                    m & 1 == 1
                } else {
                    let ws = self.wclauses[c].clone();
                    let h = self.wwords[ws.start as usize..ws.end as usize]
                        .iter()
                        .any(|w| {
                            present
                                .get(usize::from(*w) / 64)
                                .is_some_and(|x| x >> (*w % 64) & 1 == 1)
                        });
                    self.wmemo[c] = stamp | u32::from(h);
                    h
                };
                if !held {
                    all = false;
                    break;
                }
            }
            if all {
                return true;
            }
        }
        false
    }
}

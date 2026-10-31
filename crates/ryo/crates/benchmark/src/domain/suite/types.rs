//! Immutable inputs and independently pinned shipped defaults.
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Shared project inputs for one language benchmark.
#[derive(Clone, Debug)]
pub struct Suite {
    /// The language identifier shared by the project inputs.
    pub language: String,
    /// Pinned repositories used for the common corpus.
    pub projects: Vec<Project>,
    /// Exact resource collector executable used for every contestant.
    pub instrumentation: Instrumentation,
}

/// Pinned external resource collection, shared across linters in one language.
#[derive(Clone, Debug)]
pub struct Instrumentation {
    /// Absolute path to GNU time on the benchmark machine.
    pub executable: PathBuf,
    /// Approved executable digest, checked before each observed command.
    pub sha256: String,
}

/// One repository pinned to an immutable commit and a deterministic edit.
#[derive(Clone, Debug)]
pub struct Project {
    /// Stable project identifier used to name its isolated checkout.
    pub name: String,
    /// Git remote from which the pinned commit is fetched.
    pub repository: String,
    /// Full immutable Git commit identifier.
    pub revision: String,
    /// Edit patch path relative to the language benchmark directory.
    pub patch: String,
}

/// The independently pinned reference executable and its shipped defaults.
#[derive(Clone, Debug)]
pub struct Tool {
    /// Reference executable identity reported by its version command.
    pub tool: String,
    /// Exact reference version required by this suite.
    pub version: String,
    /// Expected SHA-256 digest of the reference executable.
    pub sha256: String,
    /// Exact shipped-default rule identifiers for the pinned reference profile.
    pub defaults: Vec<String>,
    /// Deferred default rules mapped to their capability-gap Beads issue.
    pub deferred: BTreeMap<String, String>,
}

/// The correctness cases authored beside one candidate query.
#[derive(Clone, Debug)]
pub struct Cases {
    /// Named positive and negative cases for the enclosing rule.
    pub cases: Vec<Case>,
}

/// An isolated set of source files and its exact expected findings.
#[derive(Clone, Debug)]
pub struct Case {
    /// Human-readable identity of this correctness case.
    pub name: String,
    /// Relative source paths and their complete UTF-8 contents.
    pub files: BTreeMap<String, String>,
    /// Expected finding multiset; an empty list is a negative case.
    pub expected: Vec<Expected>,
}

/// One expected finding in a case, located by UTF-8 byte offsets.
#[derive(Clone, Debug)]
pub struct Expected {
    /// Relative path of a source supplied by the case.
    pub file: String,
    /// Inclusive start offset in UTF-8 bytes.
    pub start: usize,
    /// Exclusive end offset in UTF-8 bytes.
    pub end: usize,
}

/// Both contestants receive exactly this list, never independently walk the tree.
#[derive(Clone, Debug)]
pub struct RunRequest {
    /// Root of the exact source snapshot supplied to both contestants.
    pub root: PathBuf,
    /// Shared explicit list of relative source paths.
    pub files: Vec<String>,
    /// Directory beneath which contestants keep their separate caches.
    pub cache: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Default coverage produced by auditing a suite, never fabricated by a caller.
///
/// ```compile_fail
/// use ryo_benchmark::domain::suite::types::Audit;
/// let forged = Audit { implemented: Vec::new(), missing: Vec::new() };
/// ```
pub struct Audit {
    pub(crate) implemented: Vec<String>,
    pub(crate) missing: Vec<String>,
}
impl Audit {
    /// Number of default rules with an authored query and valid case envelope.
    pub fn authored_count(&self) -> usize {
        self.implemented.len()
    }
    /// Authored subset; these rules still require executable correctness cases.
    pub fn authored(&self) -> &[String] {
        &self.implemented
    }
    /// Freeze an explicit nonempty subset; unselected authored rules remain outside coverage.
    pub fn select(
        &self,
        rules: &[String],
    ) -> Result<Self, super::super::diagnostics::errors::Error> {
        let selected: std::collections::BTreeSet<_> = rules.iter().cloned().collect();
        if selected.is_empty()
            || selected.len() != rules.len()
            || selected.iter().any(|rule| !self.implemented.contains(rule))
        {
            return Err(super::super::diagnostics::errors::Error::Invalid(
                "selected subset must contain distinct authored default rules".into(),
            ));
        }
        let mut missing = self.missing.clone();
        missing.extend(
            self.implemented
                .iter()
                .filter(|rule| !selected.contains(*rule))
                .cloned(),
        );
        missing.sort();
        Ok(Self {
            implemented: selected.into_iter().collect(),
            missing,
        })
    }
}

/// Evidence issued after actual cases pass, bound to exact artifacts and executables.
#[derive(Clone, Debug)]
pub struct Certificate {
    pub(crate) coverage: Audit,
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) fingerprint: String,
    pub(crate) contestants: [PathBuf; 2],
    pub(crate) cases: usize,
}

/// Whether the full inventory has been assessed or this is an interim authored subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageScope {
    /// Every default is implemented or linked to an assessed capability deferral.
    Accounted,
    /// Unassessed defaults remain explicit and the certificate covers only authored rules.
    Interim,
}
impl Certificate {
    /// Stable content identity of profile, rule/case bytes and contestant executables.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    /// Number of independently authored cases that both contestants passed.
    pub fn cases(&self) -> usize {
        self.cases
    }
}

/// Inputs supplied to the SDK worker inside the measured child.
#[derive(Clone, Debug)]
pub struct WorkerRequest {
    /// Root of the source snapshot read by the candidate.
    pub root: PathBuf,
    /// Directory holding the selected candidate rule directories.
    pub rules: PathBuf,
    /// Explicit relative paths the worker must check.
    pub files: Vec<String>,
    /// Optional persistent SDK cache directory; absence disables disk caching.
    pub cache: Option<PathBuf>,
}

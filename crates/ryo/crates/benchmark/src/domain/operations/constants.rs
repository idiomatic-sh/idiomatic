//! The words and names of the gated operations.
/// The collector verb of a language bench: verification, then the corpus, as one document.
pub(crate) const BENCH: &str = "bench";
/// The directory under the cargo target that holds every operation's output.
pub(crate) const BENCHMARK: &str = "benchmark";
/// The first component of a bench directory.
pub(crate) const LANGUAGES: &str = "languages";
/// The third component of a bench directory.
pub(crate) const BENCHMARKS: &str = "benchmarks";
/// The fourth component of a bench directory.
pub(crate) const LINTERS: &str = "linters";
/// How many components end a bench directory: `languages/<lang>/benchmarks/linters/<linter>`.
pub(crate) const BENCH_COMPONENTS: usize = 5;
/// The sealed copy of the bench's worker.
pub(crate) const WORKER: &str = "worker";
/// The sealed copy of the collector.
pub(crate) const COLLECTOR: &str = "collector";
/// The registration a bench stages.
pub(crate) const REGISTRATION: &str = "registration.json";
/// The state directory of a measure.
pub(crate) const STATE: &str = "state";
/// The directory a bench's verification writes its summary, its receipts and each rule's times to.
pub(crate) const INVENTORY: &str = "inventory";
/// The option naming the registration a measure is certified against.
pub(crate) const CASE_INVENTORY: &str = "--case-inventory=";
/// The option naming the directory the verification left its receipts and each rule's times in.
pub(crate) const VERIFICATION: &str = "--verification=";
/// The refusal of a path that is not a bench directory.
pub(crate) const NOT_A_BENCH_DIRECTORY: &str =
    "expected a bench directory languages/<lang>/benchmarks/linters/<linter>";

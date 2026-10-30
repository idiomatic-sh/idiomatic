// ignore: idiomatic.arch/file-length - the `--timings` tree is one presentation concern: it collects the spans, attributes them to crate, hex and part, prints one table over one private model, and writes the Chrome trace where no product reads it; the cli spec gives `timings` one file.
//! `--timings`: entered wall time attributed to crate, hex and port method.
//!
//! Busy time is the union of one span's entries; self wall time excludes the union
//! of its active direct children, intersected with its own entries. These are wall
//! attributions, not CPU measurements. Rows sum separate spans, so parallel and
//! nested rows may exceed the command region's wall time. The external distribution
//! runner measures whole-process wall and CPU, including startup and shutdown.
#[cfg(all(feature = "phase-diagnostic", target_os = "linux"))]
mod phases;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing_subscriber::layer::{Layer as _, SubscriberExt as _};

#[cfg(test)]
#[path = "timings_test.rs"]
mod tests;

/// How `--timings` reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Timings {
    /// No subscriber or report.
    #[default]
    Off,
    /// Time spent in each crate, in each part of it and in each call, as a tree on stderr.
    Tree,
    /// A Chrome trace file in the system's temporary directory, named on stderr, for drilling into a run: never in
    /// the product, where it would be one more input of the tools it times.
    Chrome,
    /// The outer phases of the command and when each began and ended, on stderr: a diagnostic build only.
    #[cfg(all(feature = "phase-diagnostic", target_os = "linux"))]
    Phases,
}

/// The global `--timings[=tree|chrome]` argument.
#[derive(Clone, Copy, Debug, Default, clap::Args)]
pub struct TimingsArgs {
    /// Report entered wall time on stderr, or write a Chrome trace.
    #[arg(long, global = true, value_enum, default_value = "off", num_args = 0..=1, require_equals = true, default_missing_value = "tree")]
    pub timings: Timings,
}

/// Keeps reporting alive until the command exits.
#[derive(Debug)]
pub enum TimingsGuard {
    /// Reporting is disabled.
    Off,
    /// The wall attribution tree and the command entry timestamp, before parsing.
    Tree(TimingsTree, Instant),
    /// The trace writer's guard, which flushes it when dropped, and the trace's path, said once it is written.
    Chrome(Box<dyn std::any::Any + Send>, std::path::PathBuf),
    /// The phase table printer, which prints when it is dropped.
    #[cfg(all(feature = "phase-diagnostic", target_os = "linux"))]
    Phases(Box<dyn std::any::Any + Send>),
}

impl Drop for TimingsGuard {
    fn drop(&mut self) {
        if let Self::Tree(tree, started) = self {
            tree.print(started.elapsed(), &mut std::io::stderr());
        }
        if let Self::Chrome(guard, path) = self {
            // The writer's guard flushes the trace as it drops, before the trace is named.
            drop(std::mem::replace(guard, Box::new(())));
            // Nothing is left to say when stderr is gone.
            drop(std::io::Write::write_fmt(
                &mut std::io::stderr(),
                format_args!("wrote the Chrome trace to {}\n", path.display()),
            ));
        }
    }
}

/// Installs the requested subscriber. `started` marks command entry, before parsing.
///
/// # Errors
///
/// Returns an error if another global subscriber is already installed.
pub fn install(
    timings: Timings,
    started: Instant,
) -> Result<TimingsGuard, tracing::subscriber::SetGlobalDefaultError> {
    install_with_events(timings, started, &[])
}

/// Installs timings and an explicit diagnostic event allowlist on stderr.
///
/// # Errors
///
/// Returns an error if another global subscriber is already installed.
pub fn install_with_events(
    timings: Timings,
    started: Instant,
    targets: &'static [&'static str],
) -> Result<TimingsGuard, tracing::subscriber::SetGlobalDefaultError> {
    match timings {
        Timings::Off => {
            // The progress line alone, or the diagnostic events alone, install only what they need; a command off a
            // terminal with neither installs nothing.
            let progress = crate::output::progress::layer();
            if !targets.is_empty() || progress.is_some() {
                tracing::subscriber::set_global_default(
                    tracing_subscriber::registry()
                        .with(progress)
                        .with((!targets.is_empty()).then(|| events(targets, std::io::stderr))),
                )?;
            }
            Ok(TimingsGuard::Off)
        }
        Timings::Tree => {
            let tree = TimingsTree::default();
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry()
                    .with(tracing_subscriber::filter::LevelFilter::INFO)
                    .with(tree.clone())
                    .with(events(targets, std::io::stderr))
                    .with(crate::output::progress::layer()),
            )?;
            Ok(TimingsGuard::Tree(tree, started))
        }
        Timings::Chrome => {
            let path = trace_path();
            let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
                .file(&path)
                .build();
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry()
                    .with(tracing_subscriber::filter::LevelFilter::INFO)
                    .with(layer)
                    .with(events(targets, std::io::stderr))
                    .with(crate::output::progress::layer()),
            )?;
            Ok(TimingsGuard::Chrome(Box::new(guard), path))
        }
        #[cfg(all(feature = "phase-diagnostic", target_os = "linux"))]
        Timings::Phases => phases::install().map(TimingsGuard::Phases),
    }
}

/// Where a Chrome trace is written: the system's temporary directory, outside every product, named by when and by
/// which process.
pub(crate) fn trace_path() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    std::env::temp_dir().join(format!(
        "asymptote-trace-{nanos}-{}.json",
        std::process::id()
    ))
}

/// The layer that prints the progress events of `targets` to `writer`: their messages alone, at info and above.
fn events<S, W>(targets: &'static [&'static str], writer: W) -> impl tracing_subscriber::Layer<S>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
    W: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Send + Sync + 'static,
{
    tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_timer(())
        .with_target(false)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::filter_fn(move |metadata| {
            metadata.is_event()
                && *metadata.level() <= tracing::Level::INFO
                && targets.contains(&metadata.target())
        }))
}

/// One line of the report: a crate's, a hex's or a method's spans summed.
#[derive(Clone, Copy, Debug, Default)]
struct TimingsRow {
    /// How many spans closed.
    calls: u64,
    /// The wall time the spans were entered, overlaps counted once.
    busy: Duration,
    /// The busy time no entered child span covered.
    own: Duration,
}

impl TimingsRow {
    /// Sums `other` into this row.
    fn add(&mut self, other: Self) {
        self.calls += other.calls;
        self.busy += other.busy;
        self.own += other.own;
    }
}

/// A bounded online union: overlapping entries and distinct active children count once.
#[derive(Debug, Default)]
struct Clock {
    /// The wall time the span has been entered so far.
    busy: Duration,
    /// The part of `busy` with no child entered.
    own: Duration,
    /// How many entries of the span are open, across threads.
    depth: usize,
    /// How many direct children are entered now.
    children: usize,
    /// When the clock last advanced, since the tree's origin.
    last: Duration,
}

impl Clock {
    /// Counts the time since the last transition as busy, and as own when no child is entered.
    fn advance(&mut self, now: Duration) {
        let elapsed = now.saturating_sub(self.last);
        if self.depth > 0 {
            self.busy += elapsed;
            if self.children == 0 {
                self.own += elapsed;
            }
        }
        self.last = now;
    }

    /// Opens one entry; true when it is the span's first open one.
    fn enter(&mut self, now: Duration) -> bool {
        self.advance(now);
        self.depth += 1;
        self.depth == 1
    }

    /// Closes one entry; true when no entry of the span is left open.
    fn exit(&mut self, now: Duration) -> bool {
        self.advance(now);
        self.depth -= 1;
        self.depth == 0
    }

    /// Counts a direct child becoming entered, or no longer entered.
    fn child(&mut self, entered: bool, now: Duration) {
        self.advance(now);
        if entered {
            self.children += 1;
        } else {
            self.children -= 1;
        }
    }
}

/// A live span: its clock, its parent and the row it sums into.
#[derive(Debug)]
struct SpanClock {
    /// The span's entered wall time.
    clock: Clock,
    /// The id of the span it opened under, if any.
    parent: Option<u64>,
    /// The crate, hex and method it is reported under.
    key: (String, String, String),
}

/// What the tree has seen: the live spans, and the rows of the closed ones.
#[derive(Debug, Default)]
struct State {
    /// The spans open now, by id: an id is live once, so a span inserted replaces only a closed, recycled one.
    spans: BTreeMap<u64, SpanClock>,
    /// The closed spans summed by crate, hex and method.
    rows: BTreeMap<(String, String, String), TimingsRow>,
}

impl State {
    /// Enters or exits span `id` at `now`, and tells its parent when the span's first entry opened or its last closed.
    fn transition(&mut self, id: u64, entered: bool, now: Duration) {
        let Some(span) = self.spans.get_mut(&id) else {
            return;
        };
        let changed = if entered {
            span.clock.enter(now)
        } else {
            span.clock.exit(now)
        };
        let parent = span.parent;
        if changed && let Some(parent) = parent.and_then(|up| self.spans.get_mut(&up)) {
            parent.clock.child(entered, now);
        }
    }
}

/// Aggregates entered wall time. A single lock orders transitions across threads; memory grows with live spans and
/// distinct rows, never with the number of entries.
#[derive(Clone, Debug)]
pub struct TimingsTree {
    /// The spans and rows, shared by the layer and the guard that prints them.
    state: Arc<Mutex<State>>,
    /// The instant the tree's clocks count from.
    origin: Instant,
}

impl Default for TimingsTree {
    fn default() -> Self {
        Self {
            state: Arc::default(),
            origin: Instant::now(),
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for TimingsTree
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    fn on_new_span(
        &self,
        _attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let Some(span) = ctx.span(id) else { return };
        let mut target = span.metadata().target().split("::");
        let live = SpanClock {
            clock: Clock::default(),
            parent: span.parent().map(|parent| parent.id().into_u64()),
            key: (
                target.next().unwrap_or_default().to_owned(),
                target.next().unwrap_or_default().to_owned(),
                span.name().to_owned(),
            ),
        };
        if let Ok(mut state) = self.state.lock() {
            state.spans.extend([(id.into_u64(), live)]);
        }
    }

    fn on_enter(&self, id: &tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Ok(mut state) = self.state.lock() {
            state.transition(id.into_u64(), true, self.origin.elapsed());
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Ok(mut state) = self.state.lock() {
            state.transition(id.into_u64(), false, self.origin.elapsed());
        }
    }

    fn on_close(&self, id: tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Ok(mut state) = self.state.lock()
            && let Some(span) = state.spans.remove(&id.into_u64())
        {
            state.rows.entry(span.key).or_default().add(TimingsRow {
                calls: 1,
                busy: span.clock.busy,
                own: span.clock.own,
            });
        }
    }
}

impl TimingsTree {
    /// Writes summed span wall attribution, ordered by self wall time at each level. The report is written once,
    /// whole; a writer that refuses it has no one left to tell, as the command is ending.
    pub fn print(&self, wall: Duration, out: &mut impl std::io::Write) {
        type Hexes<'a> = BTreeMap<&'a str, (TimingsRow, Vec<(&'a str, TimingsRow)>)>;
        let Ok(state) = self.state.lock() else {
            return;
        };
        let mut crates: BTreeMap<&str, (TimingsRow, Hexes<'_>)> = BTreeMap::new();
        for ((owner, hex, method), row) in &state.rows {
            let (crate_row, hexes) = crates.entry(owner.as_str()).or_default();
            crate_row.add(*row);
            let (hex_row, methods) = hexes.entry(hex.as_str()).or_default();
            hex_row.add(*row);
            methods.push((method.as_str(), *row));
        }
        let mut lines = vec![
            format!(
                "WALL attribution; command-entry region {:.6} s (not whole-process CPU)",
                wall.as_secs_f64()
            ),
            "Rows sum spans; parallel/nested rows may exceed region wall time.".to_owned(),
            format!(
                "{:<40} {:>8} {:>10} {:>10} {:>7}",
                "crate / hex / method", "calls", "busy wall", "self wall", "share"
            ),
        ];
        let mut crates: Vec<_> = crates.into_iter().collect();
        crates.sort_by_key(|entry| std::cmp::Reverse(entry.1.0.own));
        for (owner, (crate_row, hexes)) in crates {
            lines.push(line(owner, 0, crate_row, wall));
            let mut hexes: Vec<_> = hexes.into_iter().collect();
            hexes.sort_by_key(|entry| std::cmp::Reverse(entry.1.0.own));
            for (hex, (hex_row, mut methods)) in hexes {
                lines.push(line(hex, 1, hex_row, wall));
                methods.sort_by_key(|entry| std::cmp::Reverse(entry.1.own));
                for (method, method_row) in methods {
                    lines.push(line(method, 2, method_row, wall));
                }
            }
        }
        lines.push(String::new());
        // The write is std's; a refused one leaves nothing to report it to.
        drop(out.write_all(lines.join("\n").as_bytes()));
    }
}

/// One row, indented by `depth`, with its share of the command's wall time.
fn line(name: &str, depth: usize, row: TimingsRow, wall: Duration) -> String {
    let share = if wall.is_zero() {
        0.0_f64
    } else {
        row.busy.as_secs_f64() / wall.as_secs_f64() * 100.0_f64
    };
    let label = format!("{}{name}", "  ".repeat(depth));
    format!(
        "{label:<40} {:>8} {:>8.3} s {:>8.3} s {share:>6.1}%",
        row.calls,
        row.busy.as_secs_f64(),
        row.own.as_secs_f64()
    )
}

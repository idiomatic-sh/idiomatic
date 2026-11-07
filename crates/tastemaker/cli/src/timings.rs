// ignore: idiomatic.arch/file-length - the `--timings` tree is one presentation concern: it collects the spans, attributes them to crate, hex and part, and prints one table over one private model, and the cli spec gives `timings` one file; the docs every private item now carries took it past the limit.
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
use std::io::Write as _;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing_subscriber::layer::{Layer as _, SubscriberExt as _};

#[cfg(test)]
#[path = "timings_test.rs"]
mod tests;

/// The file `--timings=chrome` writes in the system's temporary directory, the latest run's.
const TRACE_FILE: &str = concat!(env!("CARGO_PKG_NAME"), "-trace.json");

/// How `--timings` reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Timings {
    /// No subscriber or report.
    #[default]
    Off,
    /// Time spent in each crate, in each part of it and in each call, as a tree on stderr.
    Tree,
    /// A Chrome trace file in the system's temporary directory, named on stderr, for drilling into a run.
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
    /// The trace writer's guard, which flushes it when dropped; held only to be dropped.
    Chrome(Box<dyn std::any::Any + Send>),
    /// The phase table printer, which prints when it is dropped.
    #[cfg(all(feature = "phase-diagnostic", target_os = "linux"))]
    Phases(Box<dyn std::any::Any + Send>),
}

impl Drop for TimingsGuard {
    fn drop(&mut self) {
        if let Self::Tree(tree, started) = self {
            // The report is written once, whole, as the command ends; a stderr that refuses it leaves no one to tell.
            drop(tree.print(started.elapsed(), &mut std::io::stderr()));
        }
    }
}

/// What a run installed: the report it prints as it ends, and, when the process already had a subscriber, this thread's
/// own for the run; both are held until the command exits.
#[derive(Debug)]
#[expect(
    dead_code,
    reason = "both fields are held only to be dropped as the command exits: the report prints, and the run's own subscriber gives way"
)]
pub struct Installed {
    /// The report, printed when it is dropped.
    report: TimingsGuard,
    /// This thread's subscriber for the run, when the process already had one: dropping it restores the one before.
    scope: Option<tracing::dispatcher::DefaultGuard>,
}

/// Makes `subscriber` the process's subscriber, or, when one is already set (a test process runs many commands), this
/// thread's for the run: a command never fails because of tracing.
pub(crate) fn installed<S>(subscriber: S) -> Option<tracing::dispatcher::DefaultGuard>
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    let dispatch = tracing::Dispatch::new(subscriber);
    tracing::dispatcher::set_global_default(dispatch.clone())
        .err()
        .map(|_already| tracing::dispatcher::set_default(&dispatch))
}

/// Installs the requested subscriber. `started` marks command entry, before parsing.
#[must_use]
pub fn install(timings: Timings, started: Instant) -> Installed {
    install_with_events(timings, started, &[])
}

/// Installs timings and an explicit diagnostic event allowlist on stderr.
#[must_use]
pub fn install_with_events(
    timings: Timings,
    started: Instant,
    targets: &'static [&'static str],
) -> Installed {
    match timings {
        Timings::Off => {
            // The warnings always, the progress line on a terminal, and the diagnostic events asked for: a layer's own
            // filter keeps every span and event it does not take out of the subscriber.
            let scope = installed(
                tracing_subscriber::registry()
                    .with(crate::output::warnings())
                    .with(crate::output::progress::layer())
                    .with((!targets.is_empty()).then(|| events(targets, std::io::stderr))),
            );
            Installed {
                report: TimingsGuard::Off,
                scope,
            }
        }
        Timings::Tree => {
            let tree = TimingsTree::default();
            let scope = installed(
                tracing_subscriber::registry()
                    .with(tracing_subscriber::filter::LevelFilter::INFO)
                    .with(tree.clone())
                    .with(events(targets, std::io::stderr))
                    .with(crate::output::warnings())
                    .with(crate::output::progress::layer()),
            );
            Installed {
                report: TimingsGuard::Tree(tree, started),
                scope,
            }
        }
        Timings::Chrome => {
            // The trace goes outside the project the command reads, and stderr says where; a stderr that refuses the line
            // leaves no one to tell.
            let trace = std::env::temp_dir().join(TRACE_FILE);
            drop(writeln!(
                std::io::stderr(),
                "the Chrome trace is written to {}",
                trace.display()
            ));
            let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
                .file(trace)
                .build();
            let scope = installed(
                tracing_subscriber::registry()
                    .with(tracing_subscriber::filter::LevelFilter::INFO)
                    .with(layer)
                    .with(events(targets, std::io::stderr))
                    .with(crate::output::warnings())
                    .with(crate::output::progress::layer()),
            );
            Installed {
                report: TimingsGuard::Chrome(Box::new(guard)),
                scope,
            }
        }
        #[cfg(all(feature = "phase-diagnostic", target_os = "linux"))]
        Timings::Phases => {
            let (subscriber, report) = phases::install();
            Installed {
                report: TimingsGuard::Phases(report),
                scope: installed(subscriber),
            }
        }
    }
}

/// The layer that prints the events of `targets` to `writer`: only those, at info and above.
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

/// One line of the report: the spans of a crate, a hex or a method, summed.
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
    /// The spans open now, by id.
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

/// Aggregates entered wall time. A single lock orders transitions across threads;
/// memory grows with live spans and distinct rows, never with the number of entries.
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
        _attributes: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let Some(span) = ctx.span(id) else { return };
        let mut target = span.metadata().target().split("::");
        let key = (
            target.next().unwrap_or_default().to_owned(),
            target.next().unwrap_or_default().to_owned(),
            span.name().to_owned(),
        );
        let parent = span.parent().map(|parent| parent.id().into_u64());
        if let Ok(mut state) = self.state.lock() {
            state.spans.extend([(
                id.into_u64(),
                SpanClock {
                    clock: Clock::default(),
                    parent,
                    key,
                },
            )]);
        }
    }

    fn on_enter(
        &self,
        id: &tracing::span::Id,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.transition(id.into_u64(), true, self.origin.elapsed());
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, _context: tracing_subscriber::layer::Context<'_, S>) {
        if let Ok(mut state) = self.state.lock() {
            state.transition(id.into_u64(), false, self.origin.elapsed());
        }
    }

    fn on_close(&self, id: tracing::span::Id, _context: tracing_subscriber::layer::Context<'_, S>) {
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
    /// Writes summed span wall attribution, ordered by self wall time at each level.
    ///
    /// # Errors
    ///
    /// Returns the writer's error if the report cannot be written.
    pub fn print(&self, wall: Duration, out: &mut impl std::io::Write) -> std::io::Result<()> {
        type Hexes<'a> = BTreeMap<&'a str, (TimingsRow, Vec<(&'a str, TimingsRow)>)>;
        let Ok(state) = self.state.lock() else {
            return Ok(());
        };
        let mut crates: BTreeMap<&str, (TimingsRow, Hexes<'_>)> = BTreeMap::new();
        for ((owner, hex, method), row) in &state.rows {
            let (crate_row, hexes) = crates.entry(owner.as_str()).or_default();
            crate_row.add(*row);
            let (hex_row, methods) = hexes.entry(hex.as_str()).or_default();
            hex_row.add(*row);
            methods.push((method.as_str(), *row));
        }
        writeln!(
            out,
            "WALL attribution; command-entry region {:.6} s (not whole-process CPU)",
            wall.as_secs_f64()
        )?;
        writeln!(
            out,
            "Rows sum spans; parallel/nested rows may exceed region wall time."
        )?;
        writeln!(
            out,
            "{:<40} {:>8} {:>10} {:>10} {:>7}",
            "crate / hex / method", "calls", "busy wall", "self wall", "share"
        )?;
        let mut crates: Vec<_> = crates.into_iter().collect();
        crates.sort_by_key(|entry| std::cmp::Reverse(entry.1.0.own));
        for (owner, (crate_row, hexes)) in crates {
            line(out, owner, 0, crate_row, wall)?;
            let mut hexes: Vec<_> = hexes.into_iter().collect();
            hexes.sort_by_key(|entry| std::cmp::Reverse(entry.1.0.own));
            for (hex, (hex_row, mut methods)) in hexes {
                line(out, hex, 1, hex_row, wall)?;
                methods.sort_by_key(|entry| std::cmp::Reverse(entry.1.own));
                for (method, method_row) in methods {
                    line(out, method, 2, method_row, wall)?;
                }
            }
        }
        Ok(())
    }
}

/// Writes one report row: `name` indented by `depth`, with its calls, times and share of `wall`.
fn line(
    out: &mut impl std::io::Write,
    name: &str,
    depth: usize,
    row: TimingsRow,
    wall: Duration,
) -> std::io::Result<()> {
    let share = if wall.is_zero() {
        0.0_f64
    } else {
        row.busy.as_secs_f64() / wall.as_secs_f64() * 100.0_f64
    };
    let label = format!("{}{name}", "  ".repeat(depth));
    writeln!(
        out,
        "{label:<40} {:>8} {:>8.3} s {:>8.3} s {share:>6.1}%",
        row.calls,
        row.busy.as_secs_f64(),
        row.own.as_secs_f64()
    )
}

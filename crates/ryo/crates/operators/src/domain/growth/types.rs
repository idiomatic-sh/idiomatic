//! The types of the `growth` concept: what every operator of the physical plan declares of how its rows grow, and the instances
//! of it a growth test runs.

/// An operator of the physical plan: what it declares of how its rows grow. A node type enters the plan's one list of operators
/// only through this contract, so an operator cannot be planned without its declaration.
///
/// # Examples
///
/// The positive control, so the refusals below are not refusals of something else:
///
/// ```
/// use ryo_operators::domain::growth::types::{Growth, Operator};
/// struct Declared;
/// impl Operator for Declared {
///     fn growth(&self) -> Growth {
///         Growth::AtMostInput
///     }
/// }
/// assert_eq!(Declared.growth(), Growth::AtMostInput);
/// ```
///
/// A new operator without its declaration does not compile:
///
/// ```compile_fail,E0046
/// use ryo_operators::domain::growth::types::Operator;
/// struct NewOp;
/// impl Operator for NewOp {}
/// ```
///
/// A node type put in the one list without the contract does not compile:
///
/// ```compile_fail,E0277
/// #[derive(Clone, Debug, PartialEq)]
/// struct Bare;
/// ryo_operators::lowerings! { Bare(Bare) }
/// ```
pub trait Operator {
    /// How this instance's rows grow: the one declaration, with no default, so an operator without it does not compile.
    fn growth(&self) -> Growth;
}

/// How an operator's output grows with its inputs: its class, and for a keyed or banded operator the fan its instance answers.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::growth::types::{Fan, Growth};
/// assert_ne!(Growth::Keyed(Fan::PerLeft), Growth::Banded(Fan::PerLeft));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Growth {
    /// At most the rows of its inputs together: a filter, a projection, an extension, a distinct, a lookup, a render, a scan.
    AtMostInput,
    /// At most the file's text: rows per row bounded by the bytes they read, as `words` and `source-matches` write. The file
    /// is the partition the runner gives every keyed kernel (today one file).
    PerRowText,
    /// At most one row per left row: an ordered match.
    OnePerLeftRow,
    /// A probe of a key, answering this fan: by its mode and the key's proven fan.
    Keyed(Fan),
    /// A band join, answering this fan: by its answer and the key's proven fan.
    Banded(Fan),
    /// At most the starts times the depth of a walk: a reach, an axis, an ordered walk, a worklist, the components.
    PerStartWalk,
    /// At most one row per group of its input.
    GroupOutput,
}

kinds! {
    /// How many rows of the right side a keyed or banded operator writes for each left row.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::growth::types::Fan;
    /// assert_ne!(Fan::PerLeft, Fan::FileProduct);
    /// ```
    pub enum Fan {
        /// At most one row, or the ties of one row: a semi, anti or ordered probe, a unique key, a nearest band.
        PerLeft,
        /// The right rows sharing its key, which repeat a bounded number of times.
        Repeats,
        /// Every right row of its file: a key of the file alone, a product of the file's rows. The file is the partition the
        /// runner gives every keyed kernel (today one file).
        FileProduct,
        /// Every pair, as the query declared with `all-pairs join`.
        DeclaredPairs,
    }
}

/// A degree of a size variable in a bound: `rows^2` has the rows degree 2.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Degree(pub(crate) u8);

/// A bound as a monomial `rows^a · depth^b · repeats^c`: the rows of the partition, the depth of a walk, and how often a shared
/// key repeats.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Bound {
    /// The degree in rows.
    pub(crate) rows: Degree,
    /// The degree in a walk's depth.
    pub(crate) depth: Degree,
    /// The degree in a shared key's repeats.
    pub(crate) repeats: Degree,
}

/// What a node writes, and the work it does to write it, each as a bound.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct NodeBound {
    /// The rows it writes.
    pub(crate) output: Bound,
    /// The work it does.
    pub(crate) work: Bound,
}

/// The instances of an operator a growth test runs: the test-side companion of `Operator`, which the plan's one list requires of
/// every node type in a test build, so an operator cannot be listed without its instances.
#[cfg(test)]
pub(crate) trait Witnesses: Sized {
    /// Its instances over a growth test's generic columns: one for each instance variant its `growth` reads, each with the role
    /// of each input it reads, in the order it reads them; its `n`th source reads the plan's `n`th node.
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>>;
}

/// The text every text cell of a growth test's generic rows names: an instance that looks a text up, or ranks it, names it.
#[cfg(test)]
pub(crate) const WITNESS_TEXT: &str = "w";

/// The columns a growth test's generic rows hold, by what each means: an operator's instances read their keys, orders, edges,
/// texts and spans at these places.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct WitnessColumns {
    /// The key two inputs are joined or grouped on.
    pub(crate) key: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// A number rows are ordered by.
    pub(crate) order: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The entity a row is of.
    pub(crate) id: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The source of an edge.
    pub(crate) parent: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The destination of an edge.
    pub(crate) child: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// A text.
    pub(crate) text: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// A span of the file.
    pub(crate) span: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// A state of a small catalogue, as a worklist's lattice holds: a catalogue does not grow with the file.
    pub(crate) state: ryo_values::domain::plan_parts::types::ColumnIndex,
}

/// What an input of an instance holds, so a growth test generates its rows: plain rows, the edges of a graph, or the entities
/// a walk admits.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum InputRole {
    /// Plain rows.
    Rows,
    /// The edges of a graph, from `parent` to `child`.
    Edges,
    /// The entities a walk admits, or the lattice a worklist reads.
    Domain,
}

/// One instance of an operator a growth test runs: the node, and the role of each input it reads.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Witnessed<N> {
    /// The instance.
    pub(crate) node: N,
    /// The role of each input it reads, in the order it reads them.
    pub(crate) roles: Vec<InputRole>,
}

/// How many rows a making may write for each row it was fed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowsPerRow(pub(crate) usize);

/// What one making may write: `per_fed_row` rows for each row it was fed, and `floor` rows whatever it was fed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RunBudget {
    /// The rows it may write for each row it was fed.
    pub(crate) per_fed_row: RowsPerRow,
    /// The rows it may write whatever it was fed.
    pub(crate) floor: ryo_values::domain::rows::types::RowCount,
}

/// What a unit's makings of a node read and wrote: the rows they were fed, every input summed, and the rows they wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Making {
    /// The rows they were fed.
    pub(crate) fed: ryo_values::domain::rows::types::RowCount,
    /// The rows they wrote.
    pub(crate) wrote: ryo_values::domain::rows::types::RowCount,
}

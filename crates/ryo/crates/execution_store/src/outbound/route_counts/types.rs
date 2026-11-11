//! The types of the `route_counts` port: plain data, public fields and derives (`Clone`, `Debug`, `PartialEq`), with no constructor and no inherent impl, and no handle, file, lock or `dyn`: a handle is a trait in interface.rs.

/// One value of a route record, in no format: an adapter writes it in its own.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::{
///     RecordKey, RecordTally, RouteField, RouteValue,
/// };
/// let row = RouteValue::Record(vec![RouteField {
///     key: RecordKey("calls"),
///     value: RouteValue::Tally(RecordTally(2)),
/// }]);
/// assert_ne!(row, RouteValue::Absent);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteValue {
    /// No value.
    Absent,
    /// A yes or a no.
    Flag(RecordFlag),
    /// An index or an identity.
    Count(RecordCount),
    /// A running total.
    Tally(RecordTally),
    /// A text.
    Text(RecordText),
    /// Values in order.
    List(Vec<RouteValue>),
    /// Named values in order.
    Record(Vec<RouteField>),
}

/// A yes or a no of a route record: whether the route's owner holds what it counts.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::RecordFlag;
/// assert!(RecordFlag(true).0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordFlag(pub bool);

/// An index or an identity in a route record.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::RecordCount;
/// assert_eq!(RecordCount(3).0, 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordCount(pub usize);

/// A running total in a route record.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::RecordTally;
/// assert_eq!(RecordTally(2).0, 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordTally(pub u64);

/// A text in a route record.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::RecordText;
/// assert_eq!(RecordText("owner_build".to_owned()).0, "owner_build");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordText(pub String);

/// The name of one value of a route record.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::RecordKey;
/// assert_eq!(RecordKey("route").0, "route");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordKey(pub &'static str);

/// One named value of a record.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::{
///     RecordKey, RecordText, RouteField, RouteValue,
/// };
/// let field = RouteField {
///     key: RecordKey("route"),
///     value: RouteValue::Text(RecordText("owner_build".to_owned())),
/// };
/// assert_eq!(field.key, RecordKey("route"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteField {
    /// The name of the value.
    pub key: RecordKey,
    /// The value.
    pub value: RouteValue,
}

/// One written document, carried as one tracing field.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::RouteDocument;
/// assert_eq!(RouteDocument("[]".to_owned()).to_string(), "[]");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteDocument(pub String);

impl std::fmt::Display for RouteDocument {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(&self.0)
    }
}

//! The types of the `route_counts` port: plain data, public fields and derives (`Clone`, `Debug`, `PartialEq`), with no constructor and no inherent impl, and no handle, file, lock or `dyn`: a handle is a trait in interface.rs.

/// One value of a route record, in no format: an adapter writes it in its own.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::{RouteField, RouteValue};
/// let row = RouteValue::Record(vec![RouteField {
///     key: "calls",
///     value: RouteValue::Tally(2),
/// }]);
/// assert_ne!(row, RouteValue::Absent);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteValue {
    /// No value.
    Absent,
    /// A yes or a no.
    Flag(bool),
    /// An index or an identity.
    Count(usize),
    /// A running total.
    Tally(u64),
    /// A text.
    Text(String),
    /// Values in order.
    List(Vec<RouteValue>),
    /// Named values in order.
    Record(Vec<RouteField>),
}

/// One named value of a record.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::types::{RouteField, RouteValue};
/// let field = RouteField {
///     key: "route",
///     value: RouteValue::Text("owner_build".to_owned()),
/// };
/// assert_eq!(field.key, "route");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteField {
    /// The name of the value.
    pub key: &'static str,
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

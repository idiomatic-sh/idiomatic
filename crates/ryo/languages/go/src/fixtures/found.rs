//! The nodes of `go/ast` a service of the domain finds over the library's tree of a source, for the cases of the services that build them.

/// The kinds of the nodes of `go/ast` that `build` finds over the library's tree of `source`, in the order it finds them: what one
/// service of the domain finds, and nothing of the others.
///
/// # Panics
///
/// When the source cannot be read, which never happens for a go file.
#[cfg(test)]
pub(crate) fn found_by(
    source: &'static str,
    build: impl FnOnce(
        &crate::domain::grammar::types::Concrete<'_>,
        &mut crate::domain::pieces::types::Pieces,
    ),
) -> String {
    use crate::domain::syntax::service::kind_name;
    use crate::domain::syntax::types::Kind;
    let tree =
        crate::outbound::syntax::fixtures::library::concreted(source).expect("the source is read");
    let mut pieces = crate::domain::pieces::types::Pieces::default();
    build(&tree, &mut pieces);
    pieces
        .nodes
        .iter()
        .map(|placed| kind_name(Kind::Node(placed.kind)).0)
        .collect::<Vec<_>>()
        .join(" ")
}

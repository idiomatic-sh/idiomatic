//! The `contracts` service: free functions named by verb that write the queries of the shared relations the comparison
//! reads, sort a file's rows by relation, tell how a contract is compared, and compare a placed file's declarations with
//! those of the content its template renders, with no IO and no ports. The declarations of code are the `declarations`
//! concept's, of a keyed document the `keyed` concept's, the blocks of a document the `blocks` concept's.

use super::constants::{
    ANNOTATES, DECORATES, DEFINES, ENTRY, MAPPING, PROSE, SCOPE, SEQUENCE_ELEMENT, SPACE,
    STRING_LITERAL, VISIBILITY, WITHIN,
};
use super::types::{
    Columns, ComparedText, ComparisonQuery, DataRow, Declaration, Decorations, Facts, Holding,
    Meeting, Mismatch, PartText, Said, Shape, Syntax, TemplateIdentity,
};
use crate::domain::comments::types::ByteSpan;
use crate::domain::definitions::types::Exactness;

/// The columns of the relation `syntax` reads.
#[must_use]
pub fn columns(syntax: Syntax) -> Columns {
    match syntax {
        Syntax::Defines => DEFINES,
        Syntax::Scopes => SCOPE,
        Syntax::Decorates => DECORATES,
        Syntax::Visibility => VISIBILITY,
        Syntax::Annotates => ANNOTATES,
        Syntax::Entries => ENTRY,
        Syntax::Mappings => MAPPING,
        Syntax::Elements => SEQUENCE_ELEMENT,
        Syntax::Prose => PROSE,
        Syntax::Literals => STRING_LITERAL,
    }
}

/// The query that reads one shared relation, whole, keyed by its node.
#[must_use]
pub fn data_query(syntax: Syntax) -> ComparisonQuery {
    let Columns { relation, id, .. } = columns(syntax);
    ComparisonQuery(format!(
        "let rows = select * from \"{}\";\nemit rows subject \"file\" key(\"{}\");\n",
        relation.0, id.0
    ))
}

/// A file's `rows`, one list for each relation.
#[must_use]
pub fn facts(rows: &[DataRow]) -> Facts<'_> {
    let mut facts = Facts::default();
    for row in rows {
        match row.syntax {
            Syntax::Defines => facts.defines.push(row),
            Syntax::Scopes => facts.scopes.push(row),
            Syntax::Decorates => facts.decorates.push(row),
            Syntax::Visibility => facts.visibility.push(row),
            Syntax::Annotates => facts.annotates.push(row),
            Syntax::Entries => facts.entries.push(row),
            Syntax::Mappings => facts.mappings.push(row),
            Syntax::Elements => facts.elements.push(row),
            Syntax::Prose => facts.prose.push(row),
            Syntax::Literals => facts.literals.push(row),
        }
    }
    facts
}

/// How a content that answered `rendered` is compared: as data when it holds keys or elements, in order when it is an
/// exact document, by its declarations when it has scopes or declarations, and blind when it answered none of these.
#[must_use]
pub fn shape(rendered: &Facts<'_>, exact: Exactness) -> Shape {
    if !rendered.entries.is_empty() || !rendered.elements.is_empty() {
        Shape::Data
    } else if exact != Exactness::Contains && !rendered.prose.is_empty() {
        Shape::Document
    } else if !rendered.scopes.is_empty() || !rendered.defines.is_empty() {
        Shape::Code
    } else {
        Shape::Blind
    }
}

/// The text of `span` in `text`, or nothing when it is not there.
#[must_use]
pub fn written(text: ComparedText<'_>, span: ByteSpan) -> PartText<'_> {
    PartText(text.0.get(span.start.0..span.end.0).unwrap_or_default())
}

/// Whether `outer` holds `inner`.
#[must_use]
pub fn holding(outer: ByteSpan, inner: ByteSpan) -> Holding {
    if outer.start <= inner.start && inner.end <= outer.end {
        Holding::Holds
    } else {
        Holding::Apart
    }
}

/// Whether `expected`, from a template, is met by `found`, from a file: the same kind, name and owners, every one of the
/// template's attributes, and, where the template's declaration names its decorations, no decoration it does not name.
#[must_use]
pub(crate) fn meeting(expected: &Declaration, found: &Declaration) -> Meeting {
    let decorated = match (&expected.decorations, &found.decorations) {
        (Decorations::Named(allowed), Decorations::Named(made)) => made.is_subset(allowed),
        (Decorations::Open, _) | (_, Decorations::Open) => true,
    };
    if expected.kind == found.kind
        && expected.name == found.name
        && expected.owners == found.owners
        && expected.attributes.is_subset(&found.attributes)
        && decorated
    {
        Meeting::Met
    } else {
        Meeting::Unmet
    }
}

/// How the declarations `found` in a file differ from those its template's content makes, `expected`: each the content
/// makes that the file does not meet, and, for an exact template, each the file makes that none the content makes is
/// met by: an exact file declares nothing more, though what it declares may carry more attributes.
#[must_use]
pub fn compare(expected: &[Declaration], found: &[Declaration], exact: Exactness) -> Vec<Mismatch> {
    let met = |wanted: &Declaration, made: &Declaration| meeting(wanted, made) == Meeting::Met;
    let mut mismatches = expected
        .iter()
        .filter(|wanted| !found.iter().any(|made| met(wanted, made)))
        .cloned()
        .map(Mismatch::Missing)
        .collect::<Vec<_>>();
    if exact != Exactness::Contains {
        mismatches.extend(
            found
                .iter()
                .filter(|made| !expected.iter().any(|wanted| met(wanted, made)))
                .cloned()
                .map(Mismatch::Extra),
        );
    }
    mismatches
}

/// A declaration as a message shows it: its attributes, its kind, its name and the declarations it is in.
#[must_use]
pub fn shown(declaration: &Declaration) -> Said {
    let mut path = declaration.name.0.to_string();
    for owner in declaration.owners.iter().rev() {
        path.push_str(WITHIN);
        path.push_str(&owner.0);
    }
    Said(
        declaration
            .attributes
            .iter()
            .map(|attribute| &*attribute.0)
            .chain([&*declaration.kind.0, &*path])
            .filter(|piece| !piece.is_empty())
            .collect::<Vec<_>>()
            .join(SPACE),
    )
}

/// The message of a mismatch with the template entry `identity`.
#[must_use]
pub fn message(mismatch: &Mismatch, identity: &TemplateIdentity) -> Said {
    let identity = &identity.0;
    Said(match mismatch {
        Mismatch::Missing(declaration) => format!(
            "`{}` from the `{identity}` template is missing",
            shown(declaration).0
        ),
        Mismatch::Extra(declaration) => format!(
            "`{}` is not in the exact `{identity}` template",
            shown(declaration).0
        ),
    })
}

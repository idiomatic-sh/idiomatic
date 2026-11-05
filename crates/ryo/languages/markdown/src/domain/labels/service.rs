//! The `labels` service: free functions named by verb that find the anchors in raw HTML, with no IO and no ports. An anchor is the value
//! of an `id` attribute of any tag, or of the `name` attribute of an `a`. They are found one at a time, so that a reader of a file
//! allocates nothing for them.

use crate::domain::nodes::types::MarkdownNode;
use ryo::languages::{self, Tree};

use super::constants::{
    ANCHOR_TAG, EQUALS, ID, NAME, NAME_END, NO_VALUE, NOT_AN_OPENING_TAG, QUOTES, SLASH, TAG_CLOSE,
    TAG_OPEN,
};
use super::types::{
    AnchorText, Attribute, AttributeName, AttributeValue, Defined, HtmlText, LabelText, Tag,
};
use crate::domain::syntax::service::{kind, labelled, level_of, named_by, written};
use crate::domain::syntax::types::Kind;

/// The first tag of `html`, or none when it holds no more.
fn next_tag(html: HtmlText<'_>) -> Option<Tag<'_>> {
    let open = html.0.find(TAG_OPEN)?;
    let after_open = html.0.get(open + TAG_OPEN.len_utf8()..)?;
    let close = after_open.find(TAG_CLOSE).unwrap_or(after_open.len());
    Some(Tag {
        inner: HtmlText(after_open.get(..close).unwrap_or_default()),
        after: HtmlText(
            after_open
                .get(close + TAG_CLOSE.len_utf8()..)
                .unwrap_or_default(),
        ),
    })
}

/// The tags of `html`, in the order they are written.
fn tags_of(html: HtmlText<'_>) -> impl Iterator<Item = Tag<'_>> {
    std::iter::successors(next_tag(html), |previous| next_tag(previous.after))
}

/// The first attribute of the text of a tag, or none when it holds no more.
fn next_attribute(rest: HtmlText<'_>) -> Option<Attribute<'_>> {
    let rest = rest
        .0
        .trim_start_matches(|each| char::is_ascii_whitespace(&each) || each == SLASH);
    let name_end = rest
        .find(|each| char::is_ascii_whitespace(&each) || NAME_END.contains(&each))
        .unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    let (name, after) = rest.split_at(name_end);
    let after = after.trim_start();
    let Some(value_start) = after.strip_prefix(EQUALS) else {
        return Some(Attribute {
            name: AttributeName(name),
            value: AttributeValue(NO_VALUE),
            after: HtmlText(after),
        });
    };
    let value_start = value_start.trim_start();
    let (value, remaining) = if let Some(quote) = value_start
        .chars()
        .next()
        .filter(|first| QUOTES.contains(first))
    {
        let inner = value_start.get(quote.len_utf8()..).unwrap_or_default();
        let close = inner.find(quote).unwrap_or(inner.len());
        (
            inner.get(..close).unwrap_or_default(),
            inner.get(close + quote.len_utf8()..).unwrap_or_default(),
        )
    } else {
        let end = value_start
            .find(|each| char::is_ascii_whitespace(&each))
            .unwrap_or(value_start.len());
        value_start.split_at(end)
    };
    Some(Attribute {
        name: AttributeName(name),
        value: AttributeValue(value),
        after: HtmlText(remaining),
    })
}

/// The attributes of the text of a tag, after its name, in the order they are written.
fn attributes_of(rest: HtmlText<'_>) -> impl Iterator<Item = Attribute<'_>> {
    std::iter::successors(next_attribute(rest), |previous| {
        next_attribute(previous.after)
    })
}

/// The anchors the raw HTML `html` gives, in the order they are written.
pub(crate) fn anchors_in(html: HtmlText<'_>) -> impl Iterator<Item = AnchorText<'_>> {
    tags_of(html)
        .filter(|tag| !tag.inner.0.starts_with(NOT_AN_OPENING_TAG))
        .flat_map(|tag| {
            let name_end = tag
                .inner
                .0
                .find(|each| !char::is_alphanumeric(each))
                .unwrap_or(tag.inner.0.len());
            let (tag_name, rest) = tag.inner.0.split_at(name_end);
            let is_anchor_tag = tag_name.eq_ignore_ascii_case(ANCHOR_TAG);
            attributes_of(HtmlText(rest)).filter_map(move |attribute| {
                let names_a_place = attribute.name.0.eq_ignore_ascii_case(ID)
                    || (is_anchor_tag && attribute.name.0.eq_ignore_ascii_case(NAME));
                (names_a_place && !attribute.value.0.is_empty())
                    .then_some(AnchorText(attribute.value.0))
            })
        })
}

/// What the element at `at` defines: a heading a section, a definition of a link reference or of a footnote its label, raw HTML the
/// anchors it gives.
#[must_use]
pub(crate) fn defined<'a>(tree: &Tree<'a>, at: languages::Position) -> Defined<'a> {
    let of = kind(tree, at);
    if of == Kind::Node(MarkdownNode::Definition) {
        labelled(tree, at).map_or(Defined::Nothing, |label| Defined::Label(LabelText(label.0)))
    } else if of == Kind::Node(MarkdownNode::FootnoteDefinition) {
        named_by(tree, at).map_or(Defined::Nothing, |label| Defined::Label(LabelText(label.0)))
    } else if of == Kind::Node(MarkdownNode::Html) || of == Kind::Node(MarkdownNode::Html) {
        Defined::Html(HtmlText(written(tree, at).0))
    } else if level_of(tree, at).is_some() {
        Defined::Section
    } else {
        Defined::Nothing
    }
}

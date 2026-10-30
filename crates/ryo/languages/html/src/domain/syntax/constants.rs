//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// Text the parser could not read.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_TEXT: &str =
    "text the parser could not read as any token: `syntax_error` reads what the library said of it";

/// DOM containers and doctypes are retained in syntax; semantic readers use the file scope and content nodes.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const DOM_STRUCTURE: &str = "native DOM containers and doctypes are represented by syntax_tree and syntax_child; semantic readers use file scopes and content nodes";
/// The DOM scalar properties of a node are nodes under it, with no bytes of their own; the structure holds them.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PROPERTY_STRUCTURE: &str = "a DOM scalar property (a name, a namespace, a prefix, a value, a doctype identifier, a document mode) as a node under its node, its value as its text: syntax_tree and syntax_child hold it, its role in syntax_child.field";
/// Text is read in runs: `prose` joins the text of the element that holds it, so no relation has a row for a text node of its own.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TEXT_RUN: &str = "text is read in runs: prose gives one row for the element that holds it, with its text joined and its code masked";
/// Lexical tokens reconstruct source; semantic readers use the tree builder's decoded DOM fields.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const LEXICAL_STRUCTURE: &str = "HTML tokenizer leaves preserve source spelling; semantic facts come from the native DOM, not duplicate lexical leaves";

/// HTML vocabulary spelling `article`.
pub(crate) const ARTICLE: &str = "article";
/// HTML vocabulary spelling `aside`.
pub(crate) const ASIDE: &str = "aside";
/// HTML vocabulary spelling `nav`.
pub(crate) const NAV: &str = "nav";
/// HTML vocabulary spelling `section`.
pub(crate) const SECTION: &str = "section";
/// The HTML vocabulary spellings of the headings, `h1` to `h6`: the first one a section holds is its title.
pub(crate) const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
/// What stands between two words of a title.
pub(crate) const WORD_SPACE: ryo::languages::NameText<'static> = ryo::languages::NameText(" ");
/// HTML vocabulary spelling `script`.
pub(crate) const SCRIPT: &str = "script";
/// HTML vocabulary spelling `style`.
pub(crate) const STYLE: &str = "style";
/// HTML vocabulary spelling `template`.
pub(crate) const TEMPLATE: &str = "template";
/// HTML vocabulary spelling `id`.
pub(crate) const ID: &str = "id";
/// HTML vocabulary spelling `name`.
pub(crate) const NAME: &str = "name";
/// HTML vocabulary spelling `a`, the anchor element.
pub(crate) const ANCHOR: &str = "a";
/// HTML vocabulary spelling `http://www.w3.org/1999/xhtml`.
pub(crate) const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
/// HTML vocabulary spelling `href`.
pub(crate) const HREF: &str = "href";
/// HTML vocabulary spelling `area`.
pub(crate) const AREA: &str = "area";
/// HTML vocabulary spelling `base`.
pub(crate) const BASE: &str = "base";
/// HTML vocabulary spelling `link`.
pub(crate) const LINK: &str = "link";
/// HTML vocabulary spelling `src`.
pub(crate) const SRC: &str = "src";
/// HTML vocabulary spelling `audio`.
pub(crate) const AUDIO: &str = "audio";
/// HTML vocabulary spelling `embed`.
pub(crate) const EMBED: &str = "embed";
/// HTML vocabulary spelling `iframe`.
pub(crate) const IFRAME: &str = "iframe";
/// HTML vocabulary spelling `img`.
pub(crate) const IMG: &str = "img";
/// HTML vocabulary spelling `input`.
pub(crate) const INPUT: &str = "input";
/// HTML vocabulary spelling `source`.
pub(crate) const SOURCE: &str = "source";
/// HTML vocabulary spelling `track`.
pub(crate) const TRACK: &str = "track";
/// HTML vocabulary spelling `video`.
pub(crate) const VIDEO: &str = "video";
/// HTML vocabulary spelling `action`.
pub(crate) const ACTION: &str = "action";
/// HTML vocabulary spelling `form`.
pub(crate) const FORM: &str = "form";
/// HTML vocabulary spelling `poster`.
pub(crate) const POSTER: &str = "poster";
/// HTML vocabulary spelling `cite`.
pub(crate) const CITE: &str = "cite";
/// HTML vocabulary spelling `blockquote`.
pub(crate) const BLOCKQUOTE: &str = "blockquote";
/// HTML vocabulary spelling `del`.
pub(crate) const DEL: &str = "del";
/// HTML vocabulary spelling `ins`.
pub(crate) const INS: &str = "ins";
/// HTML vocabulary spelling `q`, the inline quotation element.
pub(crate) const INLINE_QUOTE: &str = "q";
/// HTML vocabulary spelling `data`.
pub(crate) const DATA: &str = "data";
/// HTML vocabulary spelling `object`.
pub(crate) const OBJECT: &str = "object";
/// HTML vocabulary spelling `formaction`.
pub(crate) const FORMACTION: &str = "formaction";
/// HTML vocabulary spelling `button`.
pub(crate) const BUTTON: &str = "button";
/// HTML vocabulary spelling `http://www.w3.org/2000/svg`.
pub(crate) const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
/// `XLink` namespace of foreign URL attributes.
pub(crate) const XLINK_NAMESPACE: &str = "http://www.w3.org/1999/xlink";

/// An absent namespace or scalar value.
pub(crate) const EMPTY: &str = "";
/// The number a URL-valued attribute keeps its target under, resolved to a path of the run as the language contract resolves a link:
/// the adapter writes it where it builds the tree, which is given the file's path, and `reference` reads it back. It is no DOM
/// property: it is published nowhere and has no node, so the tree and its numbering are the DOM's alone. Its number, 8, is the one
/// after the DOM scalar properties, which `property_id` numbers 1 to 7 and a value under `PropertyId(0)` is the attribute's own; a
/// test holds it apart from all of them.
pub(crate) const TARGET_PROPERTY: ryo::languages::PropertyId = ryo::languages::PropertyId(8);
/// HTML vocabulary spelling `code`: its text is code, which a reader of prose does not read.
pub(crate) const CODE: &str = "code";
/// HTML vocabulary spelling `br`: a break between two words is a space.
pub(crate) const BREAK: &str = "br";
/// The HTML elements whose text continues the run around them (phrasing content that is text); every other element holds a run of its
/// own.
pub(crate) const INLINE: [&str; 34] = [
    "a", "abbr", "b", "bdi", "bdo", "big", "cite", "code", "data", "del", "dfn", "em", "font", "i",
    "ins", "kbd", "label", "mark", "q", "rp", "rt", "ruby", "s", "samp", "small", "span", "strike",
    "strong", "sub", "sup", "time", "tt", "u", "var",
];

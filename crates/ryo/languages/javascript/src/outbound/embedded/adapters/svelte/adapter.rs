//! The svelte adapter: the instance and module `<script>` blocks of a `.svelte` file. Template expressions and styles are not read.

use super::constants::{
    CLOSE, COMMENT_CLOSE, COMMENT_OPEN, EXTENSION, LANG, OPEN, SELF_CLOSE, TAG_END, TYPED,
};
use crate::outbound::embedded::interface::EmbeddedPort;
use crate::outbound::embedded::types::{Block, Blocks, Dialect, Markup, Start};
use ryo::languages::{Recognition, SourcePath};

/// Finds the script blocks of a `.svelte` file.
#[derive(Debug)]
pub(crate) struct EmbeddedSvelteAdapter;

/// The value of the `lang` attribute, however it is written: `lang="ts"`, `lang='ts'`, `lang=ts`, white space around the `=` allowed.
fn lang_of(attributes: Markup<'_>) -> Option<Markup<'_>> {
    let mut rest = attributes.0;
    while let Some((before, after)) = rest.split_once(LANG) {
        let named_alone = before.chars().next_back().is_none_or(char::is_whitespace);
        rest = after;
        if !named_alone {
            continue;
        }
        let Some(value) = after.trim_start().strip_prefix('=') else {
            continue;
        };
        let value = value.trim_start();
        let mut characters = value.chars();
        return Some(Markup(match characters.next() {
            Some(quote @ ('"' | '\'')) => {
                characters.as_str().split(quote).next().unwrap_or_default()
            }
            Some(_) | None => value.split(char::is_whitespace).next().unwrap_or_default(),
        }));
    }
    None
}

/// The dialect a script tag's attributes name: `lang="ts"` and `lang="tsx"` are typed, anything else is script.
fn dialect(attributes: Markup<'_>) -> Dialect {
    if lang_of(attributes).is_some_and(|lang| TYPED.contains(&lang.0)) {
        Dialect::Typed
    } else {
        Dialect::Script
    }
}

impl EmbeddedPort for EmbeddedSvelteAdapter {
    fn claims(&self, path: SourcePath<'_>) -> Recognition {
        if path.0.ends_with(EXTENSION) {
            Recognition::Reads
        } else {
            Recognition::Declines
        }
    }

    fn blocks<'a>(&self, text: Markup<'a>) -> Blocks<'a> {
        let source = text.0;
        let mut found = Vec::new();
        let mut at = 0;
        while let Some(offset) = source.get(at..).and_then(|tail| tail.find(OPEN)) {
            let tag = at + offset;
            at = tag + OPEN.len();
            let (Some(before), Some(rest)) = (source.get(..tag), source.get(at..)) else {
                break;
            };
            let boundary = rest.starts_with(char::is_whitespace) || rest.starts_with(TAG_END);
            let commented = before
                .rfind(COMMENT_OPEN)
                .and_then(|open| before.get(open..))
                .is_some_and(|comment| !comment.contains(COMMENT_CLOSE));
            if !boundary || commented {
                continue;
            }
            let Some((attributes, _)) = rest.split_once(TAG_END) else {
                break;
            };
            let start = at + attributes.len() + TAG_END.len();
            at = start;
            if attributes.ends_with(SELF_CLOSE) {
                continue;
            }
            let Some((script, _)) = source.get(start..).and_then(|tail| tail.split_once(CLOSE))
            else {
                break;
            };
            found.push(Block {
                text: Markup(script),
                start: Start(start),
                dialect: dialect(Markup(attributes)),
            });
            at = start + script.len() + CLOSE.len();
        }
        Blocks(found)
    }
}

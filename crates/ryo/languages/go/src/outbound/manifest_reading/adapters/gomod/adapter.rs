//! The manifest reading port, implemented for the manifests of Go. `go.mod` and `go.work` are made of directives, a verb and its arguments on a
//! line, or a verb and a block of lines between parentheses; a comment runs from `//` to the line end, and `// indirect` on a
//! requirement says that the module's own code does not need it. A path is a word, or a string between double quotes or backquotes.

use ryo::languages::ManifestText;

use super::constants::{
    ARROW, CLOSE, COMMENT, DELIMITERS, INDIRECT, MODULE, OPEN, QUOTES, REPLACE, REQUIRE, USE,
};
use super::types::{Block, Directive, Line, LineNumber, LineText, Word};
use crate::domain::literals::service::unquoted;
use crate::domain::literals::types::Written;
use crate::domain::modules::types::{
    Declared, Indirectness, Manifestness, Member, ModulePath, ModuleVersion, Replacement,
    Requirement, ModuleReplacementTarget,
};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;

/// The `gomod` manifest reading adapter.
#[derive(Debug)]
pub(crate) struct ManifestReadingGomodAdapter;

/// Reads the words of a line, and its comment.
fn words_of(text: LineText<'_>, number: LineNumber) -> Result<Line, ManifestReadingError> {
    let mut words = Vec::new();
    let mut indirect = Indirectness::Direct;
    let mut rest = text.0.trim_start();
    while !rest.is_empty() {
        if let Some(comment) = rest.strip_prefix(COMMENT) {
            if comment.trim() == INDIRECT {
                indirect = Indirectness::Indirect;
            }
            break;
        }
        let first = rest.chars().next().unwrap_or(' ');
        let (word, after) = if QUOTES.contains(&first) {
            // A string runs to its closing quote: in a double-quoted one a `\"` is part of it. Its text is what Go reads it as,
            // its escapes decoded (`"example.com/\u007a"` is `example.com/z`), as `modfile` does with `strconv.Unquote`.
            let inner = rest.get(first.len_utf8()..).unwrap_or_default();
            let mut escaped = false;
            let close = inner
                .char_indices()
                .find(|(_, each)| {
                    let closes = !escaped && *each == first;
                    escaped = first == '"' && !escaped && *each == '\\';
                    closes
                })
                .map(|(at, _)| at)
                .ok_or(ManifestReadingError::Unquoted { line: number.0 })?;
            let end = first.len_utf8() + close + first.len_utf8();
            let quoted = rest.get(..end).unwrap_or_default();
            let decoded = unquoted(Written(quoted))
                .ok_or(ManifestReadingError::Unquoted { line: number.0 })?;
            words.push(Word(decoded.0.into_owned()));
            rest = rest.get(end..).unwrap_or_default().trim_start();
            continue;
        } else {
            let end = if DELIMITERS.contains(first) {
                first.len_utf8()
            } else {
                rest.char_indices()
                    .find(|(_, each)| each.is_whitespace() || DELIMITERS.contains(*each))
                    .map_or(rest.len(), |(at, _)| at)
            };
            rest.split_at(end)
        };
        words.push(Word(word.to_owned()));
        rest = after.trim_start();
    }
    Ok(Line { words, indirect })
}

/// The first argument of a directive; the directive is incomplete without it.
fn first_of<'a>(
    arguments: &'a [Word],
    number: LineNumber,
    verb: &Word,
) -> Result<&'a Word, ManifestReadingError> {
    arguments
        .first()
        .ok_or_else(|| ManifestReadingError::Incomplete {
            line: number.0,
            verb: verb.0.clone(),
        })
}

/// Which directive a verb is.
fn directive_of(verb: &Word) -> Directive {
    if verb.0 == MODULE {
        Directive::Module
    } else if verb.0 == REQUIRE {
        Directive::Require
    } else if verb.0 == REPLACE {
        Directive::Replace
    } else if verb.0 == USE {
        Directive::Use
    } else {
        Directive::Other
    }
}

/// What the directive `verb` with `arguments` declares, added to `declared`.
fn declare(
    declared: &mut Declared,
    kind: Manifestness,
    verb: &Word,
    arguments: &[Word],
    number: LineNumber,
    indirect: Indirectness,
) -> Result<(), ManifestReadingError> {
    let missing = || ManifestReadingError::Incomplete {
        line: number.0,
        verb: verb.0.clone(),
    };
    match (directive_of(verb), kind) {
        (Directive::Module, Manifestness::Module) => {
            let path = first_of(arguments, number, verb)?;
            declared.module = Some(ModulePath(path.0.as_str().into()));
        }
        (Directive::Require, _) => {
            let path = first_of(arguments, number, verb)?;
            declared.requirements.push(Requirement {
                path: ModulePath(path.0.as_str().into()),
                version: ModuleVersion(
                    arguments
                        .get(1)
                        .map_or_else(Box::default, |version| version.0.as_str().into()),
                ),
                indirect,
            });
        }
        (Directive::Replace, _) => {
            let arrow = arguments
                .iter()
                .position(|each| each.0 == ARROW)
                .ok_or_else(missing)?;
            let from = first_of(arguments, number, verb)?;
            let to = arguments.get(arrow + 1).ok_or_else(missing)?;
            // `replace module [version] => target [version]`: each version is the word after its path, when there is one.
            let version_of = |word: Option<&Word>| {
                ModuleVersion(
                    word.filter(|each| each.0 != ARROW)
                        .map_or_else(Box::default, |each| each.0.as_str().into()),
                )
            };
            declared.replacements.push(Replacement {
                from: ModulePath(from.0.as_str().into()),
                version: version_of(arguments.get(1)),
                to: ModuleReplacementTarget(to.0.as_str().into()),
                to_version: version_of(arguments.get(arrow + 2)),
            });
        }
        (Directive::Use, Manifestness::Workspace) => {
            let directory = first_of(arguments, number, verb)?;
            declared.uses.push(Member(directory.0.as_str().into()));
        }
        (
            Directive::Module | Directive::Use | Directive::Other,
            Manifestness::Module | Manifestness::Workspace | Manifestness::Other,
        ) => {}
    }
    Ok(())
}

impl ManifestReadingPort for ManifestReadingGomodAdapter {
    fn declared(
        &self,
        kind: Manifestness,
        text: &ManifestText,
    ) -> Result<Declared, ManifestReadingError> {
        let mut declared = Declared::default();
        let mut block: Option<Block> = None;
        for (place, each) in text.lines().enumerate() {
            let number = LineNumber(place + 1);
            let Line {
                mut words,
                indirect,
            } = words_of(LineText(each), number)?;
            if words.is_empty() {
                continue;
            }
            // `)` closes the open block.
            if words.first().is_some_and(|word| word.0 == CLOSE) {
                if block.take().is_none() {
                    return Err(ManifestReadingError::Unopened { line: number.0 });
                }
                continue;
            }
            let (verb, arguments) = if let Some(open) = &block {
                (open.verb.clone(), words)
            } else {
                let verb = words.remove(0);
                if words.first().is_some_and(|word| word.0 == OPEN) {
                    block = Some(Block { verb, line: number });
                    continue;
                }
                (verb, words)
            };
            declare(&mut declared, kind, &verb, &arguments, number, indirect)?;
        }
        if let Some(open) = block {
            return Err(ManifestReadingError::Unclosed {
                line: open.line.0,
                verb: open.verb.0,
            });
        }
        if kind == Manifestness::Module && declared.module.is_none() {
            return Err(ManifestReadingError::NoModule);
        }
        Ok(declared)
    }
}

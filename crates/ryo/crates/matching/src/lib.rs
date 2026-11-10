#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

//! Pure matching semantics. Storage and matcher lifetime belong to the caller.
//!
//! The public inbound contract is the only callable entry point.
//!
//! ```compile_fail,E0603
//! use ryo_matching::domain::matching::service;
//! ```
//! ```compile_fail,E0432
//! use ryo_matching::application;
//! ```
//! ```compile_fail,E0432
//! use ryo_matching::outbound;
//! ```
//! ```compile_fail,E0616
//! use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::RegexSource, use_case::MatchingUseCaseImpl};
//! let compiled = MatchingUseCaseImpl::new().compile_regex(RegexSource("ok")).unwrap();
//! let _ = compiled.regex;
//! ```
//! ```compile_fail,E0616
//! use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::GlobPattern, use_case::MatchingUseCaseImpl};
//! let compiled = MatchingUseCaseImpl::new().compile_text_globs(&[GlobPattern("a?c")]);
//! let _ = compiled.program;
//! ```
//! ```
//! use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::{MatchOutcome, RegexSource, SearchText}, use_case::MatchingUseCaseImpl};
//! let matching = MatchingUseCaseImpl::new();
//! let compiled = matching.compile_regex(RegexSource("^ok$")).unwrap();
//! assert_eq!(matching.matches_regex(&compiled, SearchText("ok")), MatchOutcome::Matches);
//! assert_eq!(matching.matches_regex(&compiled, SearchText("other")), MatchOutcome::Differs);
//! ```

mod domain;
pub mod inbound;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

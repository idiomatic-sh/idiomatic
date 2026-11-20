//! pass: a language linked with its walk beside its two use cases, through the `walk:` arm of `link!`; the arm without it still
//! links.

use std::sync::Arc;

use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl;
use ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl;

fn main() {
    let walking = ryo::link!(
        "words",
        parse: Arc::new(ParseUseCaseImpl),
        project: Arc::new(ProjectUseCaseImpl),
        walk: Arc::new(Walking),
    );
    let plain = ryo::link!("words", parse: Arc::new(ParseUseCaseImpl), project: Arc::new(ProjectUseCaseImpl));
    assert_eq!(walking.name(), plain.name());
}

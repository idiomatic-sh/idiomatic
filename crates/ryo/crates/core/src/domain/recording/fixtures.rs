//! Matching values supplied to recording contract cases.
use ryo_matching::inbound::use_cases::matching::types::{GlobHits, GlobPattern};
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase, use_case::MatchingUseCaseImpl,
};
pub(super) fn matcher(
    globs: &[ryo_values::domain::files::types::Glob],
) -> Result<
    ryo_values::domain::files::types::GlobsMatcher,
    ryo_matching::inbound::use_cases::matching::errors::MatchingError,
> {
    let patterns: Vec<GlobPattern<'_>> = globs
        .iter()
        .map(|glob| GlobPattern(glob.0.as_ref()))
        .collect();
    let compiled = MatchingUseCaseImpl::new().compile_globs(&patterns)?;
    Ok(ryo_values::domain::files::types::GlobsMatcher {
        matches: std::sync::Arc::new(
            move |path: ryo_values::domain::files::types::MatchCandidate<'_>,
                  hits: &mut ryo_values::domain::files::types::Hits| {
                let mut found = GlobHits(std::mem::take(&mut hits.0));
                MatchingUseCaseImpl::new().match_globs(
                    &compiled,
                    std::path::Path::new(path.0),
                    &mut found,
                );
                hits.0 = found.0;
            },
        ),
    })
}

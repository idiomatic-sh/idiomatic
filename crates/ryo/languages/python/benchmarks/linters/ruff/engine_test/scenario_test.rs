//! The scenarios of the engine proofs.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EngineScenario {
    LexicallyCompleteSyntaxError,
    PreservedSemanticSyntaxError,
    PreservedStubSemanticSyntaxError,
    OrdinaryAnnotationNamedExpression,
    InvalidSyntaxProjectionDisabled,
    CustomFrontendProjection,
    FutureFeatureProjection,
    LateFutureDirectiveProjection,
    UnknownFutureWithoutConsumer,
    FrontendRuleCollision,
    ARejectedFileLeavesPt020Pt025AndPt026AnsweringTheValidFile,
    ARejectedFileLeavesI001Plw0406AndRuf100AnsweringTheValidFile,
    SharedSuppressionFragmentMatchesTheHistoricalInlineQueries,
    ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay,
    LazyFutureStaysVisibleWithoutADedicatedSelectedRule,
    ListedProjectInputsAndInvalidSourcesKeepDistinctReaders,
    I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory,
}

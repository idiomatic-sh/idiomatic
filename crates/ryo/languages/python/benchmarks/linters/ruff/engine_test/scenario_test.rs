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
    SharedSuppressionFragmentMatchesTheHistoricalInlineQueries,
    LazyFutureStaysVisibleWithoutADedicatedSelectedRule,
    ListedProjectInputsAndInvalidSourcesKeepDistinctReaders,
    Pie794RootedEquivalenceReportsNothingAcrossUnrelatedGrowth,
    SparseDescendantConsumersEquivalenceReportsNothingAcrossUnrelatedGrowth,
    Plr1708RootedEquivalenceReportsNothingAcrossUnrelatedGrowth,
    ActualRootedQueriesKeepTheirSparseAxis,
}

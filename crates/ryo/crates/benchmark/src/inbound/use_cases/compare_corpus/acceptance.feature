Feature: compare_corpus

  Rule: compare_corpus judges every difference by the differences accepted for its corpus

    @succeeds
    Scenario: An accepted difference passes
      Then compare_corpus passes a corpus whose one difference is accepted

    @refuses
    Scenario: The wrong rule set is refused before either contestant runs
      Then compare_corpus refuses a request that selects other than the configured rules

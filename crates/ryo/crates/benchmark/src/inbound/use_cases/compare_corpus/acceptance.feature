Feature: compare_corpus

  Rule: compare_corpus judges every difference by the differences accepted for its corpus

    @succeeds
    Scenario: An accepted difference passes
      Then compare_corpus passes a corpus whose one difference is accepted

    @refuses
    Scenario: The wrong rule set is refused before either contestant runs
      Then compare_corpus refuses a request that selects other than the configured rules

  Rule: compare_corpus compares every discovered file of a suffix the profile declares, and no other

    @succeeds
    Scenario: Every file of a declared suffix is compared, and no other
      Then compare_corpus compares a.ts and b.js and skips c.vue when the profile declares .ts .mts .cts .tsx .js .jsx .mjs .cjs

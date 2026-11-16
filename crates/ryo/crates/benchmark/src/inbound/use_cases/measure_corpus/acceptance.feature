Feature: measure_corpus
  @refuses
  Scenario: A corpus of no project publishes no cost
    Then measure_corpus refuses a corpus of no project, publishing no cost

  @succeeds
  Scenario: The corpus shape counts TypeScript and JavaScript files alike
    Then measure_corpus counts a.ts and b.mjs in the corpus shape and not c.vue

  @refuses
  Scenario: A project with nothing left to lint is refused
    Then measure_corpus refuses a project whose every file the suffix filter excludes, naming it

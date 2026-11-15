Feature: measure_corpus
  @succeeds
  Scenario: A diagnostic corpus is not a metric
    Then measure_corpus says a diagnostic corpus publishes no metric

  @refuses
  Scenario: An accounted corpus without samples refuses publication
    Then measure_corpus refuses to publish an accounted corpus without samples

  @succeeds
  Scenario: The corpus shape counts TypeScript and JavaScript files alike
    Then measure_corpus counts a.ts and b.mjs in the corpus shape and not c.vue

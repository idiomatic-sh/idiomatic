Feature: measure_corpus
  @succeeds
  Scenario: A diagnostic corpus is not a metric
    Then measure_corpus says a diagnostic corpus publishes no metric

  @refuses
  Scenario: An accounted corpus without samples refuses publication
    Then measure_corpus refuses to publish an accounted corpus without samples

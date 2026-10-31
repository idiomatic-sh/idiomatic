Feature: measure
  @succeeds
  Scenario: Both contestants agree
    Then matching reports release a measured pair

  @refuses
  Scenario: Coverage is incomplete
    Then incomplete coverage releases no measured pair

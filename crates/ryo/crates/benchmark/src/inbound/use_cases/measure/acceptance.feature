Feature: measure
  @succeeds
  Scenario: Both contestants agree
    Then matching reports release a measured pair

  @refuses
  Scenario: Certified workload changed
    Then changed certified workload releases no measured pair

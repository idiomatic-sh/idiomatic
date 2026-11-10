Feature: certify
  @succeeds
  Scenario: Unchanged evidence permits observation
    Then unchanged certification releases two verified cases
  @refuses
  Scenario: Mutated workload loses certification
    Then a changed patch refuses certified observation

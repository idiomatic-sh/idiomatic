Feature: admit_measurement
  @succeeds
  Scenario: A diagnostic run is held to nothing
    Then admit_measurement admits a diagnostic subset

  @refuses
  Scenario: An accounted run without authored rules is refused
    Then admit_measurement refuses an accounted run that authored nothing

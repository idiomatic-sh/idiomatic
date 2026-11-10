Feature: stage_suite

  @succeeds
  Scenario: Subset through the public operation
    Then stage_suite subset preserves its promised result

  @succeeds
  Scenario: Workload through the public operation
    Then stage_suite workload preserves its promised result

  @succeeds
  Scenario: Supporting through the public operation
    Then stage_suite supporting preserves its promised result

  @succeeds
  Scenario: Discrepancy through the public operation
    Then stage_suite discrepancy preserves its promised result

  @refuses
  Scenario: Missing through the public operation
    Then stage_suite missing preserves its promised result

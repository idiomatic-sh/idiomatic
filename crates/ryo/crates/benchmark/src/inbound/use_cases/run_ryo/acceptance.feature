Feature: run_ryo
  @succeeds
  Scenario: Submitted files survive the worker boundary
    Then the worker preserves the checked file list

  @fails
  Scenario: Engine failure cannot become a clean report
    Then the worker preserves an engine refusal

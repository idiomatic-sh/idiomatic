Feature: run

  Rule: a node's rows are made from the open file's facts, and a relation the facts refuse is a fault

    @succeeds
    Scenario: a select of a relation answers the rows that hold
      Given the run input "held"
      When run runs
      Then run answers "1 row"

    @refuses
    Scenario: a relation the facts refuse is never empty rows
      Given the run input "refused"
      When run runs
      Then run answers "refused NotRun"

  Rule: every making is held to the run budget, counting on or off

    @refuses
    Scenario: a making past the run budget stops the run
      Given the run input "one name 600 times"
      When run runs
      Then run answers "refused OverBudget { fed: RowCount(1200), wrote: RowCount(360000) }"

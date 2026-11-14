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

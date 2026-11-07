Feature: publication

  Rule: A producer's table is published only from a worker that owns its arena

    @succeeds
    Scenario: A numeric table is published unchanged
      Given a publication from worker "producer"
      When publication runs
      Then publication answers "published"

    @refuses
    Scenario: The reserved cross-file worker is refused
      Given a publication from worker "cross-file"
      When publication runs
      Then publication answers "refused"

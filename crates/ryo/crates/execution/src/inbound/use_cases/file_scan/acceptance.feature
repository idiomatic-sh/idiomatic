Feature: file_scan

  Rule: A request's files are scanned by the new engine with the rule set of the language that reads them

    @succeeds
    Scenario: The new engine finds the fixture rule's rows in a file its language reads
      Given a file scan request "new engine"
      When the file "a.walk" is scanned
      Then the file scan answers "2 rows"

    @succeeds
    Scenario: The old engine alone scans nothing
      Given a file scan request "old engine"
      When the file "a.walk" is scanned
      Then the file scan answers "0 rows; file not walked: the language does not read this file"

    @refuses
    Scenario: A run of the new engine at an edge that composed none is refused
      Given a file scan request "no engine"
      When the file "a.walk" is scanned
      Then the file scan answers "refused"

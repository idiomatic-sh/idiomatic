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

  Rule: A rule that passes its allowance is warned of, and fails the run only where the scan is set to

    @succeeds
    Scenario: in a user's run an overrun is handed over and the file is answered as it would be without it
      Given a file scan request "new engine, a rule overruns in b.walk"
      When the files "a.walk" and "b.walk" are scanned
      Then the file scan says
        """
        a.walk: 2 rows
        b.walk: 2 rows
        warning: rule `rule` is slow in `b.walk`
          step 2 of the rule's plan joins its left input (1000 rows) with its right input (1000 rows) with no key between them
          that is 1000000 pairs in this one file; the run continues and its findings are complete
          to fix: join on a shared column or add a range, so each row meets only its matches
        """

    @refuses
    Scenario: in a gate's run the same overrun fails the run, naming the rule and the file
      Given a file scan request "new engine, set to fail on an overrun in b.walk"
      When the files "a.walk" and "b.walk" are scanned
      Then the file scan says
        """
        a.walk: 2 rows
        b.walk: 2 rows; the run fails
        warning: rule `rule` is slow in `b.walk`
          step 2 of the rule's plan joins its left input (1000 rows) with its right input (1000 rows) with no key between them
          that is 1000000 pairs in this one file; the run continues and its findings are complete
          to fix: join on a shared column or add a range, so each row meets only its matches
        """

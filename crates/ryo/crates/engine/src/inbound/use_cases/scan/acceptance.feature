Feature: scan

  Rule: scan runs only what the file can answer

    @succeeds
    Scenario: an empty rule set scans a file and runs nothing
      Given the scan input "no rules"
      When scan runs over "one two"
      Then scan counted "0" candidates and "0" steps
      And the language filled "0" relations

    @succeeds
    Scenario: a file without a rule's anchor costs that rule no step
      Given the scan input "absent anchor kind"
      When scan runs over "one two"
      Then scan counted "0" candidates and "0" steps

    @succeeds
    Scenario: a relation no enabled program reads is never filled
      Given the scan input "two relations, one read"
      When scan runs over "one two!"
      Then the language filled "1" relations

  Rule: scan runs each operation of a rule by its one meaning

    @succeeds
    Scenario: as-of pairs each probe with the latest event before it
      Given the scan input "as-of latest before"
      When scan runs over "a b a b a"
      Then scan found "3" rows

    @succeeds
    Scenario: as-of answers every right row tied for nearest
      Given the scan input "as-of tied events"
      When scan runs over "a a b"
      Then scan found "4" rows

    @succeeds
    Scenario: as-of with bound after pairs each probe with the earliest event after it
      Given the scan input "as-of earliest after"
      When scan runs over "a a b"
      Then scan found "2" rows

    @succeeds
    Scenario: group-max keeps every row tied for the greatest order
      Given the scan input "group-max ties"
      When scan runs over "a a b"
      Then scan found "3" rows

    @succeeds
    Scenario: group-ordinal numbers a group from zero
      Given the scan input "group-ordinal from zero"
      When scan runs over "a b c"
      Then scan found "1" rows

    @succeeds
    Scenario: follow answers each source's fixed points and no row for a dead end
      Given the scan input "follow fixed points"
      When scan runs over "a b"
      Then scan found "2" rows

    @succeeds
    Scenario: written rows are made once for the run
      Given the scan input "written rows"
      When scan runs over "a b"
      Then scan found "0" rows

  Rule: a refused or partial relation is a fault, never empty rows

    @refuses
    Scenario: scan faults the rules reading a partial relation
      Given the scan input "partial relation"
      When scan runs over "one two?"
      Then scan answers "rule SelectedRuleOrdinal(0): relation SyntaxError read in part: PartialCause(\"a question the fixture cannot read\")"

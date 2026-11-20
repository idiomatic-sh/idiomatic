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

  Rule: scan decides what a file can answer before it runs a candidate

    @succeeds
    Scenario: a node whose input is empty in the file is never evaluated
      Given the scan input "join with an empty side"
      When scan runs over "one two"
      Then scan counted "0" candidates and "0" steps

    @succeeds
    Scenario: a join that is empty by its keys, though neither input is, runs no candidate
      Given the scan input "join empty by its keys"
      When scan runs over "one two"
      Then scan counted "0" candidates and made the table "1" times

  Rule: scan makes a table once and evaluates a definition once for a key

    @succeeds
    Scenario: a table is made on its first read and once, however many programs read it
      Given the scan input "one table, two readers"
      When scan runs over "a a b"
      Then scan made the table "1" times

    @succeeds
    Scenario: a definition asked the same key twice is evaluated once
      Given the scan input "one definition, two callers"
      When scan runs over "a a"
      Then scan evaluated the definition "1" times

    @succeeds
    Scenario: a read by key asks the language for that key alone
      Given the scan input "keyed read"
      When scan runs over "one two!"
      Then the language filled "1" keys and "0" relations whole

  Rule: a rule's work is counted against its rows, and an overrun is said, never stopped

    @succeeds
    Scenario: a rule whose work passes its allowance in a file still finds every row, and the engine says so
      Given the scan input "pairs within one group"
      When scan runs over a file of "1000" words in one group
      Then scan found every row and noticed "1" rule over its allowance

    @succeeds
    Scenario: a rule just under its allowance is not noticed
      Given the scan input "pairs within one group"
      When scan runs over a file of "100" words in one group
      Then scan found every row and noticed "0" rules

    @succeeds
    Scenario: the same lookup written with as-of stays within its allowance on 8,000 events
      Given the scan input "as-of latest before"
      When scan runs over a file of "8000" events and "8000" probes in one group
      Then scan noticed "0" rules

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

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

  Rule: a refused or partial relation is a fault, never empty rows

    @refuses
    Scenario: scan faults the rules reading a partial relation
      Given the scan input "partial relation"
      When scan runs over "one two?"
      Then scan answers "rule SelectedRuleOrdinal(0): relation SyntaxError read in part: PartialCause(\"a question the fixture cannot read\")"

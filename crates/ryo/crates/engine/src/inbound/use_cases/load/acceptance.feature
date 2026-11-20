Feature: load

  Rule: load makes a rule set's programs, and answers each rule it cannot load

    @succeeds
    Scenario Outline: load accepts a rule set
      Given the load input "<input>"
      When load runs
      Then load answers "<expected>"

      Examples:
        | name           | input                       | expected         |
        | empty_rule_set | no rules                    | loaded 0 rules   |
        | one_tree_rule  | one tree rule               | loaded 1 rules   |
        | unbound        | unbound correlated sub-plan | loaded 1 rules   |
        | over_budget    | program over budget         | loaded 1 rules   |

    @succeeds
    Scenario: load accepts the rules it can lower and names each one it refuses
      Given the load input "one tree rule and one over the host's rows"
      When load runs
      Then load answers "loaded 1 rules; refused 1: rule SelectedRuleOrdinal(1) not loaded: step PlanNode(0) cannot be evaluated"

    @succeeds
    Scenario Outline: a cached pack gives the same findings as a cold lowering
      Given the load input "<input>"
      When load runs through a pack
      Then load answers "<expected>"

      Examples:
        | name          | input         | expected                     |
        | one_tree_rule | one tree rule | pack and lowering run 1 rules |

  Rule: load refuses nothing for what it costs or for how large it is

    @succeeds
    Scenario: a rule that joins two relations of the file with no key between them is loaded, and its join is one table
      Given the load input "a product within a file"
      When load runs
      Then load answers "loaded 1 rules, 1 tables with a product stage"

    @succeeds
    Scenario: a definition of 65 columns is loaded
      Given the load input "a definition of 65 columns"
      When load runs
      Then load answers "loaded 1 rules"

    @refuses
    Scenario: a step of 65,536 columns is refused, naming what was counted
      Given the load input "a step of 65,536 columns"
      When load runs
      Then load answers "loaded 0 rules; refused 1: rule SelectedRuleOrdinal(0) not loaded: more than 65,535 columns in one table"

  Rule: a sub-plan is lowered once for the rules that share it

    @succeeds
    Scenario: two rules whose plans differ only in a bound value do not share a definition
      Given the load input "two rules, one sub-plan, two bound values"
      When load runs
      Then load answers "loaded 2 rules, 2 definitions"

    @succeeds
    Scenario: a sub-plan two rules share is lowered once
      Given the load input "two rules, one sub-plan"
      When load runs
      Then load answers "loaded 2 rules, 1 definitions"

  Rule: statistics change how a rule set runs, never what it finds

    @succeeds
    Scenario: statistics that do not match the rules are ignored, never refused
      Given the load input "one tree rule, statistics of other rules"
      When load runs
      Then load answers "loaded 1 rules; statistics matched 0 units"

    @succeeds
    Scenario: rules loaded with and without statistics find the same rows
      Given the load input "two orders of one rule"
      When scan runs over "one two"
      Then both programs found the same rows

  Rule: a pack reads back only for the engine that wrote it

    @refuses
    Scenario Outline: load refuses a pack of another engine
      Given the load input "<input>"
      When load runs through a pack
      Then load answers "<expected>"

      Examples:
        | name          | input         | expected                                 |
        | other_version | other version | pack written by another engine version   |

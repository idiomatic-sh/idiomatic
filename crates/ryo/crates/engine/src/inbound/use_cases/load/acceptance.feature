Feature: load

  Rule: load makes a rule set's programs or names what it cannot

    @succeeds
    Scenario Outline: load accepts a rule set
      Given the load input "<input>"
      When load runs
      Then load answers "<expected>"

      Examples:
        | name           | input          | expected         |
        | empty_rule_set | no rules       | loaded 0 rules   |
        | one_tree_rule  | one tree rule  | loaded 1 rules   |

    @succeeds
    Scenario Outline: a cached pack gives the same findings as a cold lowering
      Given the load input "<input>"
      When load runs through a pack
      Then load answers "<expected>"

      Examples:
        | name          | input         | expected                     |
        | one_tree_rule | one tree rule | pack and lowering run 1 rules |

  Rule: a rule the lowering cannot express is refused at load, named

    @refuses
    Scenario Outline: load refuses a rule
      Given the load input "<input>"
      When load runs
      Then load answers "<expected>"

      Examples:
        | name           | input                       | expected                                                       |
        | unbound        | unbound correlated sub-plan | rule SelectedRuleOrdinal(0) not loaded: Unbound(PlanNode(2))   |
        | over_budget    | program over budget         | rule SelectedRuleOrdinal(0) not loaded: program over budget    |

    @refuses
    Scenario Outline: load refuses a pack of another engine
      Given the load input "<input>"
      When load runs through a pack
      Then load answers "<expected>"

      Examples:
        | name          | input         | expected                                 |
        | other_version | other version | pack written by another engine version   |

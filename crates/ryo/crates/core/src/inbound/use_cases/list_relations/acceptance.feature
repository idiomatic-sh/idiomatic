Feature: list_relations

  Rule: list_relations answers what its caller sees

    @succeeds
    Scenario Outline: list_relations succeeds
      Given the list_relations input "<input>"
      When list_relations runs
      Then list_relations answers "<expected>"

      Examples:
        | name | input | expected |
        | catalog | works | 61 relations |

    @refuses
    Scenario Outline: list_relations refuses
      Given the list_relations input "<input>"
      When list_relations runs
      Then list_relations answers "<expected>"

      Examples:
        | name | input | expected |
        | catalog_down | fails | the relation catalog failed: the relation catalog is unavailable: the fake was told to fail |

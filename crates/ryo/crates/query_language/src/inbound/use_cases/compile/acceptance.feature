Feature: compile

  Rule: compile answers what its caller sees

    @succeeds
    Scenario Outline: compile succeeds
      Given the compile input "<input>"
      When compile runs
      Then compile answers "<expected>"

      Examples:
        | name | input | expected |
        | valid | valid | 1 steps |
        | as_of | as_of | 2 steps |
        | group_max | group_max | 2 steps |
        | group_ordinal | group_ordinal | 2 steps |
        | follow | follow | 2 steps |
        | written_rows | written_rows | 1 steps |
        | written_list | written_list | 4 steps |

    @refuses
    Scenario Outline: compile refuses
      Given the compile input "<input>"
      When compile runs
      Then compile answers "<expected>"

      Examples:
        | name | input | expected |
        | unknown_relation | unknown_relation | no relation named `nothing` (from `<query>` at byte 0) |
        | order_not_a_number | order_not_a_number | `group-max` orders by a number, and its order column `name` is not one (from `<query>` at byte 69) |
        | short_written_row | short_written_row | row 2 of `rows` has 1 cells, and `columns` names 2 (from `<query>` at byte 0) |

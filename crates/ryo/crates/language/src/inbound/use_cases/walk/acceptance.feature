Feature: walk

  Rule: walk writes a file's node table and lends it with the language's fillers

    @succeeds
    Scenario Outline: walk lends a file it reads
      Given the walk input "<input>"
      When walk runs
      Then walk answers "<expected>"

      Examples:
        | name          | input                 | expected                                     |
        | words         | words.walk:one two    | walked 1 nodes, 3 elements, 0 syntax errors  |
        | exclamation   | words.walk:one two!   | walked 1 nodes, 3 elements, 1 syntax errors  |

  Rule: walk refuses a file its language does not read

    @refuses
    Scenario Outline: walk refuses
      Given the walk input "<input>"
      When walk runs
      Then walk answers "<expected>"

      Examples:
        | name        | input            | expected                             |
        | unsupported | src/lib.rs:fn f  | the language does not read this file |

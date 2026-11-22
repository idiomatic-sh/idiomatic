Feature: walk

  Rule: walk writes a file's node table and lends it with the language's fillers

    @succeeds
    Scenario Outline: walk lends a file it reads
      Given the walk input "<input>"
      When walk runs
      Then walk answers "<expected>"

      Examples:
        | name        | input                | expected                                    |
        | words       | words.walk:one two   | walked 1 nodes, 3 elements, 0 syntax errors |
        | exclamation | words.walk:one two!  | walked 1 nodes, 3 elements, 1 syntax errors |
        | label       | words.walk:name: one | walked 1 nodes, 3 elements, 0 syntax errors |
        | quote       | words.walk:one <two> | walked 2 nodes, 6 elements, 0 syntax errors |

  Rule: a file's one request answers each relation asked, and a declared key's rows are found by the table's index

    @succeeds
    Scenario Outline: a relation of the request is answered and read by its keys
      Given the walk input "<input>"
      When walk fills "<fill>"
      Then walk answers "<expected>"

      Examples:
        | name       | input                         | fill                    | expected |
        | whole      | words.walk:one two three four | token-link whole        | TBD      |
        | by source  | words.walk:one two three four | token-link by source    | TBD      |
        | by target  | words.walk:one two three four | token-link by target    | TBD      |
        | undeclared | words.walk:one two three four | token-link by link-kind | TBD      |
        | by id      | words.walk:one! two!          | syntax-error by id      | TBD      |

  Rule: a fixed word is one cell for the whole run

    @succeeds
    Scenario: a fixed word is the same cell in every file of the run
      Given the walk input "words.walk:one two"
      When walk fills the links of the file and then of a file of "three four five"
      Then walk answers "word 1 `next-token`, then word 1 `next-token`"

  Rule: a filler is held to what the catalog declares of its relation

    @succeeds
    Scenario: a relation's rows grow with the file as its catalog item declares
      Given the walk input "words.walk:end!"
      When the growth harness holds the relations of files of 64 elements and of four times as many
      Then walk answers "64 and 256 elements: language-target PerFile 1 to 1, syntax-error PerElement 1 to 1, token-link PerElement 62 to 254; no fault"

    @rejects
    Scenario Outline: the growth harness rejects a filler weaker than the catalog declares
      Given the walk input "<input>"
      When the growth harness holds the relations of files of 64 elements and of four times as many
      Then walk answers "<expected>"

      Examples:
        | name           | input          | expected                                                                                                                                                                                                      |
        | a row per pair | words.walk:%!  | 64 and 256 elements: language-target PerFile 1 to 1, syntax-error PerElement 1 to 1, token-link PerElement 3969 to 65025; [Grew(TokenLink)]           |
        | two for a file | words.walk:+!  | 64 and 256 elements: language-target PerFile 2 to 2, syntax-error PerElement 1 to 1, token-link PerElement 62 to 254; [Grew(LanguageTarget)]                                                                  |
        | no row         | words.walk:end | 64 and 256 elements: language-target PerFile 1 to 1, syntax-error PerElement 0 to 0, token-link PerElement 62 to 254; [Unproven(SyntaxError)]                                                                 |

  Rule: walk refuses a file its language does not read

    @refuses
    Scenario Outline: walk refuses
      Given the walk input "<input>"
      When walk runs
      Then walk answers "<expected>"

      Examples:
        | name        | input           | expected                             |
        | unsupported | src/lib.rs:fn f | the language does not read this file |

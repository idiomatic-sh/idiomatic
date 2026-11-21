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

  Rule: a filler writes the rows it is asked for, or every row and says so

    @succeeds
    Scenario: a filler with no body for a key writes every row and says so
      Given the walk input "words.walk:one! two!"
      When walk fills "syntax-error by id"
      Then walk answers "every row though a key was asked, 2 rows"

    @succeeds
    Scenario Outline: a filler with a body for a key writes that key's rows
      Given the walk input "words.walk:one two three four"
      When walk fills "<fill>"
      Then walk answers "<expected>"

      Examples:
        | name       | fill                    | expected                                 |
        | whole      | token-link whole        | the rows asked, 3 rows                   |
        | by source  | token-link by source    | the rows asked, 1 rows                   |
        | by target  | token-link by target    | the rows asked, 1 rows                   |
        | undeclared | token-link by link-kind | every row though a key was asked, 3 rows |

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
      When the growth harness holds the "relations" of files of 64 elements and of four times as many
      Then walk answers "64 and 256 elements: language-target PerFile 1 to 1, syntax-error PerElement 1 to 1, token-link PerElement 62 to 254; no fault"

    @succeeds
    Scenario: a fill of a declared key writes that key's rows alone, whatever the size of the file
      Given the walk input "words.walk:end!"
      When the growth harness holds the "keys" of files of 64 elements and of four times as many
      Then walk answers "64 and 256 elements: token-link by source 1 to 1, token-link by target 1 to 1; no fault"

    @rejects
    Scenario Outline: the growth harness rejects a filler weaker than the catalog declares
      Given the walk input "<input>"
      When the growth harness holds the "<held>" of files of 64 elements and of four times as many
      Then walk answers "<expected>"

      Examples:
        | name           | input          | held      | expected                                                                                                                                                                                                      |
        | a row per pair | words.walk:%!  | relations | 64 and 256 elements: language-target PerFile 1 to 1, syntax-error PerElement 1 to 1, token-link PerElement 3969 to 65025; [Grew(TokenLink), KeyRows(TokenLink(Source)), KeyRows(TokenLink(Target))]           |
        | two for a file | words.walk:+!  | relations | 64 and 256 elements: language-target PerFile 2 to 2, syntax-error PerElement 1 to 1, token-link PerElement 62 to 254; [Grew(LanguageTarget)]                                                                  |
        | no key body    | words.walk:^!  | keys      | 64 and 256 elements: token-link by source 62 to 254, token-link by target 62 to 254; [KeyNotAsked(TokenLink(Source)), KeyGrew(TokenLink(Source)), KeyNotAsked(TokenLink(Target)), KeyGrew(TokenLink(Target))] |
        | every row      | words.walk:&!  | keys      | 64 and 256 elements: token-link by source 62 to 254, token-link by target 62 to 254; [KeyRows(TokenLink(Source)), KeyGrew(TokenLink(Source)), KeyRows(TokenLink(Target)), KeyGrew(TokenLink(Target))]         |
        | no row         | words.walk:end | relations | 64 and 256 elements: language-target PerFile 1 to 1, syntax-error PerElement 0 to 0, token-link PerElement 62 to 254; [Unproven(SyntaxError)]                                                                 |

  Rule: walk refuses a file its language does not read

    @refuses
    Scenario Outline: walk refuses
      Given the walk input "<input>"
      When walk runs
      Then walk answers "<expected>"

      Examples:
        | name        | input           | expected                             |
        | unsupported | src/lib.rs:fn f | the language does not read this file |

# crate-dependency-depth 0.1.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. The reach is now counted between manifest files, each edge joined to the set of manifest files (a projection on `file`, one row per file); a counted manifest is kept when it is a member's (joined to the members' files) and only then joined to its manifest node for the span. The findings and the scenarios are as in 0.1.0, the first release.

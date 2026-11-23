# Paired elements

`paired-elements(pairs=<target: node, value: node, file: file>, target_kinds, value_kinds)` matches two displays element by
element, at any depth: for each given pair whose target's kind matches the glob `target_kinds` and whose value's kind matches
`value_kinds` (at every level), it returns every name
`binding` inside the target with the `value` node at the same place in the value (the same element rank at every level), only
where the two shapes agree all the way down. `assigned-values` (`{tuple,list}` into `{tuple,list,set}`) gives it each assignment's or walrus's target and value, to say
what an unpacked name is bound to; PLW0127 (`{tuple}` into `{tuple}`, as Ruff compares only tuples) also gives it two targets of one chained assignment, to find a name assigned to
itself.

# constants-once 0.2.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. The constants in an adapter take their port and adapter from their own file's path (a capture over the constants), where 0.2.0 joined every file of the repository by path. The findings and the scenarios are as in 0.2.0.

# constants-once 0.2.0

A constant that sibling adapters of one port each declare is not reported when every other declaration of it is in such a sibling
adapter (`adapters/vue/` and `adapters/svelte/` of the `embedded` port, each scanning its own markup for the same script tag): one
adapter may not name another's constant (adapter-owns-no-adapter), and a markup tag is no concept of the port's domain. It is reported
as before when a file outside the sibling adapters declares it, or when two files of one adapter do.

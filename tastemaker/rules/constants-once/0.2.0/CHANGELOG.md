# constants-once 0.2.0

A constant that sibling adapters of one port each declare is not reported when every other declaration of it is in such a sibling
adapter (`adapters/vue/` and `adapters/svelte/` of the `embedded` port, each scanning its own markup for the same script tag): one
adapter may not name another's constant (adapter-owns-no-adapter), and a markup tag is no concept of the port's domain. It is reported
as before when a file outside the sibling adapters declares it, or when two files of one adapter do.

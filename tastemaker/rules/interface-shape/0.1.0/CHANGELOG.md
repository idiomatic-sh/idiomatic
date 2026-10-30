# interface-shape

## 0.1.0

The `nu-hex` spec's owned rule `interface-shape`, shared with the `adapter-crate` spec because a collection's `interface.rs` exists in both. Its query is unchanged; its text names the three interfaces.

The retired `Interface` suffix is named nowhere in the rule any more: the `part` spec places the rule over the `interface.rs` of a use case (`<Name>UseCase`) and of an outbound port (`<Name>Port`), and the namespace specs over a collection's (`<Collection>Item`). The query is unchanged; the finding texts say "part" for what they called a hex, and the fixtures name the three suffixes.

## 0.1.0 (amended)

An `interface.rs` holds one port trait plus only the traits that its methods return, the handles. A trait is a root when no method of another trait in the file returns it (its name is an identifier of that method's return type); more than one root is a finding, so a trait no method returns is a finding. A handle still needs a `self` method. The rule is shared, so a use case's and a collection's `interface.rs` may declare handles too. Fixtures: `port-with-handle`, `unreturned-extra-trait`, `handle-without-self-method`.

## 0.1.0

The value-trait arm no longer reads a collection's `interface.rs` (`.../collections/<collection>/interface.rs`): its item trait (`RelationsItem`, `RefinementsItem`) may have no `self` method, so `interface-shape` and `interface-present-unless-item-trait` no longer disagree. Fixtures `collection-item-trait` (passes) and `collection-item-trait-outside-collections` (still a finding).

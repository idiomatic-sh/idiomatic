# derived-view-acquisition

## 0.3.0

Who reaches the derived owner is decided by where the code sits, not by a list: the `allowed` param (a `{ file, name }` per caller and holder) is removed, and the new required `owner-concept` param names the owner's concept directory (`derived`). A confined identifier written in a production file beneath a directory of that name is the owner's own; anywhere else it is a finding, so acquisition is collapsed into one owner module per crate. The 0.2.0 exclusion of `application/file_dag/derived/owner/**` is that directory. Scenarios `owner-concept-may-name-the-view` (pass), `acquisition-outside-the-owner-concept` and `holder-outside-the-owner-concept` (violations) replace the three of 0.2.0 that read `allowed`. Everything else is as in 0.2.0.

## 0.2.0

Who reaches the derived owner is confined: an identifier of the new required `confined` param (the owner's acquiring entries and the leased views and handles it issues), written in a production file outside the owner tree, is a finding unless the new required `allowed` param lists that `{ file, name }`. It replaces the derived census's text scans of the acquiring entries' callers (`sites`) and of the files that hold a leased view or handle (`Holder::ALL`). Fixtures `listed-holder-and-owner-may-name-the-view` (pass), `unlisted-caller-of-an-acquiring-entry` and `unlisted-holder-of-a-leased-view` (violations). Everything else is as in 0.1.0.

## 0.1.0

Edited in place: the adapters, fake and contract of the derived port are its implementations and build through it; a file outside `outbound/derived` that names the port and calls `build` is still a consumer and is refused.

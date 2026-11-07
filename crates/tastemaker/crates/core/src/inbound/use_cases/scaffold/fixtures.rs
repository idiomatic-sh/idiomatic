//! The test support of the `scaffold` use case: the scenario its tests share.

use crate::inbound::use_cases::check::fixtures::RULE;

/// A spec whose entry passes a param that names the capture of its `include`, and whose second file waits on a condition.
pub const NAMED_SPEC: &str = "description = \"A named part.\"\nwhy = \"A test needs a param.\"\nparams = { package = { type = \"text\", description = \"The package.\" } }\nconditions = { notes = { description = \"The part has notes.\", rule = { rule = \"present\", include = [\"*.md\"] } } }\ntemplates = [{ template = \"manifest\", file = \"manifest.toml\", description = \"The manifest.\" }, { template = \"manifest\", alias = \"notes\", file = \"notes.toml\", count = { when = { condition = \"notes\" }, then = 1, else = 0 }, description = \"A file only a part with notes has.\" }]\n";
/// The manifest's template: the param written as the package's name.
pub const NAMED_TEMPLATE: &str = "description = \"The manifest.\"\nwhy = \"A test needs a template.\"\nparams = { package = { type = \"text\", description = \"The package.\" } }\ncontent = \"name = \\\"{{ package }}\\\"\\n\"\n";
/// A template whose file its tool writes: it renders nothing.
pub const GENERATED_TEMPLATE: &str =
    "description = \"A lock.\"\nwhy = \"A test needs a file a tool writes.\"\ncontent = \"\"\n";
/// A template whose content names a filter the renderer has not: it does not render.
pub const UNRENDERABLE_TEMPLATE: &str = "description = \"A broken file.\"\nwhy = \"A test needs a template that does not render.\"\ncontent = \"{{ 1 | no_such_filter }}\"\n";
/// A workspace file that is no TOML.
pub const UNREADABLE_WORKSPACE: &str = "version = [2\n";
/// The workspace: the named part at each `src/{domain}`, its package `x-{domain}`, and a manifest at each `src/{domain}.toml`.
pub const NAMED_WORKSPACE: &str = "workspace = { name = \"w\", dependencies = [{ package = \"p\", path = \"p\" }] }\nclosed = false\nspecs = [{ spec = \"p/named\", include = [\"src/{domain}\"], params = { package = \"x-{domain}\" } }]\ntemplates = [{ template = \"p/manifest\", file = \"src/{domain}.toml\", count = { min = 0 }, params = { package = \"x-{domain}\" } }, { template = \"p/generated\", file = \"src/{domain}.lock\", count = { min = 0 } }, { template = \"p/cycling\", file = \"src/{domain}.cycle\", count = { min = 0 } }, { template = \"p/unrenderable\", file = \"src/{domain}.bad\", count = { min = 0 } }]\n";
/// A workspace whose one file waits on a condition passed a param the root does not have.
pub const UNBOUND_WORKSPACE: &str = "workspace = { name = \"w\", dependencies = [{ package = \"p\", path = \"p\" }] }\nclosed = false\nconditions = { flag = { description = \"The flag is up.\", rule = { rule = \"p/marker\", include = [\"x\"], params = { max = { param = \"bound\" } } } } }\ntemplates = [{ alias = \"unbound\", template = \"p/manifest\", file = \"flag.txt\", count = { when = { condition = \"flag\" }, then = 0, else = 1 }, params = { package = \"x\" } }]\n";
/// The workspace of the language scene: one spec entry, placed by a capture of the directory.
pub const LANGUAGE_WORKSPACE: &str = "workspace = { name = \"w\", dependencies = [{ package = \"q\", path = \"q\" }] }\nclosed = false\nspecs = [{ spec = \"q/lang-part\", include = [\"langs/{lang}\"] }]\n";
/// The package of the language scene: a spec with a capture, a condition and a nested part that takes a param with a capture in it.
pub const LANGUAGE_PACKAGE: &[(&str, &str)] = &[
    ("package.toml", "name = \"q\"\n"),
    (
        "specs/lang-part/0.1.0/spec.toml",
        "description = \"A language.\"\nwhy = \"A test needs a spec with captures, a condition and a nested part.\"\nconditions = { marked = { description = \"A marker is there.\", rule = { rule = \"present\", include = [\"marker.txt\"] } } }\nspecs = [\n { spec = \"q/inner@0.1.0\", include = [\"inner\"], params = { trait = \"pkg::{lang}\" }, description = \"The inner part.\" },\n]\ntemplates = [\n { template = \"root\", file = \"root.txt\", description = \"The root.\" },\n { template = \"item\", file = \"items/{lib}.txt\", description = \"One for each library.\" },\n { template = \"extra\", file = \"extra.txt\", count = { when = { condition = \"marked\" }, then = 1, else = 0 }, description = \"Only when marked.\" },\n { template = \"marker\", file = \"marker.txt\", description = \"The marker.\" },\n { template = \"needs\", file = \"needs.txt\", description = \"A file whose template needs a param.\" },\n]\n\n[params]\nlang = { type = \"identifier\", from = \"directory-name\", description = \"The language.\" }\n",
    ),
    ("specs/lang-part/0.1.0/rules/present/rule.toml", RULE),
    (
        "specs/lang-part/0.1.0/templates/root/template.toml",
        "description = \"The root.\"\nwhy = \"A test.\"\ncontent = \"root {{ lang }}\"\n",
    ),
    (
        "specs/lang-part/0.1.0/templates/item/template.toml",
        "description = \"An item.\"\nwhy = \"A test.\"\ncontent = \"item {{ captures.lib | first }}\"\n",
    ),
    (
        "specs/lang-part/0.1.0/templates/extra/template.toml",
        "description = \"Extra.\"\nwhy = \"A test.\"\ncontent = \"extra\"\n",
    ),
    (
        "specs/lang-part/0.1.0/templates/needs/template.toml",
        "description = \"Needs.\"\nwhy = \"A test.\"\nparams = { kind = { type = \"string\", description = \"The kind.\" } }\ncontent = \"kind={{ kind }}\"\n",
    ),
    (
        "specs/lang-part/0.1.0/templates/marker/template.toml",
        "description = \"Marker.\"\nwhy = \"A test.\"\ncontent = \"marker\"\n",
    ),
    (
        "specs/inner/0.1.0/spec.toml",
        "description = \"The inner part.\"\nwhy = \"A test needs a nested spec.\"\ntemplates = [\n { template = \"body\", file = \"body.txt\", params = { trait = { param = \"trait\" } }, description = \"The body.\" },\n]\n\n[params]\ntrait = { type = \"string\", description = \"The trait.\" }\n",
    ),
    (
        "specs/inner/0.1.0/templates/body/template.toml",
        "description = \"Body.\"\nwhy = \"A test.\"\nparams = { trait = { type = \"string\", description = \"The trait.\" } }\ncontent = \"trait={{ trait }}\"\n",
    ),
];

/// The workspace of the hex scene: one spec placed by a capture of its directory, and the module file beside each instance.
pub const HEX_WORKSPACE: &str = "workspace = { name = \"w\", dependencies = [{ package = \"h\", path = \"h\" }] }\nclosed = false\nspecs = [{ spec = \"h/hexish\", include = [\"src/{domain}\"] }]\ntemplates = [{ template = \"h/module\", file = \"src/{domain}.txt\" }]\n";

/// The package of the hex scene: a spec whose first file asks for the kind and whose optional file is written for one kind.
pub const HEX_PACKAGE: &[(&str, &str)] = &[
    ("package.toml", "name = \"h\"\n"),
    (
        "specs/hexish/0.1.0/spec.toml",
        "description = \"A hex.\"\nwhy = \"A test needs a spec whose first writing is told a kind.\"\ntemplates = [\n { template = \"errors\", file = \"errors.txt\", description = \"The errors.\" },\n { template = \"ports\", file = \"ports.txt\", count = { min = 0 }, description = \"A port's file.\" },\n]\n",
    ),
    (
        "specs/hexish/0.1.0/templates/errors/template.toml",
        "description = \"Errors.\"\nwhy = \"A test.\"\ncontent = \"{%- if scaffold | default(false) and (kind is not defined or kind not in ['core', 'port']) %}{{ refuse('say the kind') }}{% endif -%}\\nerrors\"\n",
    ),
    (
        "specs/hexish/0.1.0/templates/ports/template.toml",
        "description = \"Ports.\"\nwhy = \"A test.\"\ncontent = \"{%- if scaffold | default(false) and (kind | lower) == 'port' %}ports{% endif %}\"\n",
    ),
    (
        "templates/module/0.1.0/template.toml",
        "description = \"A module file.\"\nwhy = \"A test.\"\ncontent = \"part {{ domain }}\"\n",
    ),
];

/// A template whose own presence removes its content, so staging must reject the cycle.
pub const CYCLING_TEMPLATE: &str = r#"description = "A self-negating file."
why = "The scaffold must refuse a cycle without publishing files."
content = '{% if not (present.cycling | default(false)) %}flag{% endif %}'
"#;

//! `tm validate`, over each sample: the rule `tm create` writes holds as written.

use crate::languages::{RULE, created, initialized, over_samples, printed};

#[test]
fn validate_holds_the_created_rule() {
    over_samples("validate", |sample| {
        let scratch = initialized(sample)?;
        created(&format!("rule/{RULE}"), sample, scratch.path())?;
        let printed = printed(&["validate", &format!("rule/{RULE}")], scratch.path())?;
        if printed.contains("1 version(s) hold, 0 do not") {
            Ok(())
        } else {
            Err(format!("tm validate says:\n{printed}"))
        }
    });
}

#[test]
fn a_scenario_whose_rule_reaches_no_file_fails_validation() {
    over_samples("validate unreached scenario", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        created(&format!("rule/{RULE}"), sample, project)?;
        let file = project.join(format!("tastemaker/rules/{RULE}/0.1.0/rule.toml"));
        let text = std::fs::read_to_string(&file)
            .map_err(|error| format!("the rule cannot be read: {error}"))?;
        let Some(second) =
            text.find("[[scenarios]]\nid       = \"a-file-with-text-is-not-reported\"")
        else {
            return Err(format!("the created rule has no second scenario:\n{text}"));
        };
        let mut unreached = text.get(..second).unwrap_or_default().to_owned();
        unreached.push_str("[[scenarios]]\nid       = \"nothing-is-reached\"\nscenario = \"A scenario whose include reaches no file proves nothing.\"\ngiven.include = [\"nowhere/**\"]\ngiven.files.\"src/one.txt\" = \"\"\n");
        std::fs::write(&file, unreached)
            .map_err(|error| format!("the rule cannot be written: {error}"))?;
        match printed(&["validate", &format!("rule/{RULE}")], project) {
            Ok(said) => Err(format!(
                "tm validate holds a scenario that reaches nothing:\n{said}"
            )),
            Err(_) => Ok(()),
        }
    });
}

#[test]
fn a_stray_file_in_the_package_fails_validation() {
    over_samples("validate stray", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        std::fs::write(
            project.join("tastemaker/stray.txt"),
            "a file no definition holds\n",
        )
        .map_err(|error| format!("the stray cannot be written: {error}"))?;
        match printed(&["validate"], project) {
            Ok(said) => Err(format!(
                "tm validate holds a package with a stray file:\n{said}"
            )),
            Err(_) => Ok(()),
        }
    });
}

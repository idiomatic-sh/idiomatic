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

/// The bounds for validating the repository's own definitions: under 5 s with nothing proved yet, under 1 s with everything
/// proved, and no more than 500 MB resident. The goals are 300 ms cold and 10 ms warm, and these bounds are lowered when the run reaches them.
#[cfg(target_os = "linux")]
const COLD_BOUND: std::time::Duration = std::time::Duration::from_secs(5);
#[cfg(target_os = "linux")]
const WARM_BOUND: std::time::Duration = std::time::Duration::from_secs(1);
#[cfg(target_os = "linux")]
const PEAK_BOUND_KIB: u64 = 500 * 1024;

/// Runs `tm validate` in `directory` under `time`, answering how long it took and the peak resident set it reached, in KiB.
#[cfg(target_os = "linux")]
fn validated(directory: &std::path::Path) -> Result<(std::time::Duration, u64), String> {
    let started = std::time::Instant::now();
    let output = std::process::Command::new("/usr/bin/time")
        .args(["-f", "peak-kib %M", env!("CARGO_BIN_EXE_tm"), "validate"])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("tm cannot start: {error}"))?;
    let elapsed = started.elapsed();
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        return Err(format!("tm validate failed:\n{said}"));
    }
    let peak = said
        .lines()
        .find_map(|line| line.strip_prefix("peak-kib "))
        .and_then(|kib| kib.trim().parse::<u64>().ok())
        .ok_or_else(|| format!("`time` reported no peak:\n{said}"))?;
    Ok((elapsed, peak))
}

/// Validating the definitions of the repository itself stays within the bounds above: cold, warm and in memory. The bounds are limits, not
/// what is measured today.
#[test]
#[cfg(target_os = "linux")]
fn validating_the_repository_s_definitions_stays_within_its_bounds() {
    let scratch = tempfile::tempdir().expect("the scene directory");
    let git = std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(scratch.path())
        .status()
        .expect("git starts");
    assert!(git.success(), "git init succeeds");
    crate::fixes::copy(
        &crate::fixes::root().join("tastemaker"),
        &scratch.path().join("tastemaker"),
    )
    .expect("the definitions copied");
    std::fs::write(
        scratch.path().join("tastemaker.toml"),
        concat!(
            "tastemaker = \"",
            env!("CARGO_PKG_VERSION"),
            "\"\nworkspace = { name = \"bar\", dependencies = [{ package = \"idiomatic.arch\", path = \"tastemaker\" }] }\n"
        ),
    )
    .expect("the workspace written");
    let (cold, cold_peak) = validated(scratch.path()).expect("a cold validation");
    let (warm, warm_peak) = validated(scratch.path()).expect("a warm validation");
    let mut missed = Vec::new();
    if cold > COLD_BOUND {
        missed.push(format!("cold {cold:?} is over {COLD_BOUND:?}"));
    }
    if warm > WARM_BOUND {
        missed.push(format!("warm {warm:?} is over {WARM_BOUND:?}"));
    }
    for (which, peak) in [("cold", cold_peak), ("warm", warm_peak)] {
        if peak > PEAK_BOUND_KIB {
            missed.push(format!(
                "{which} peak {peak} KiB is over {PEAK_BOUND_KIB} KiB"
            ));
        }
    }
    assert!(missed.is_empty(), "{}", missed.join("; "));
}

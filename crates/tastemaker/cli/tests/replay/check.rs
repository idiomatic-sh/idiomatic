//! `tm check`, over each sample: the rule `tm create` writes reports where its own failing scenario says.

use crate::fixes::{FOUND, tm};
use crate::languages::{
    RULE, copied, created, created_rule, ended, initialized, over_samples, ran, reported,
};

/// The first finding the failing scenario of `rule` expects: its path and line, and the text of the file it is given.
fn failing_case(rule: &toml::Table) -> Option<(String, Option<i64>, String)> {
    let scenario = rule.get("scenarios")?.as_array()?.iter().find(|scenario| {
        scenario
            .get("then")
            .and_then(|then| then.get("findings"))
            .and_then(toml::Value::as_array)
            .is_some_and(|findings| !findings.is_empty())
    })?;
    let expected = scenario.get("then")?.get("findings")?.as_array()?.first()?;
    let path = expected.get("path")?.as_str()?.to_owned();
    let line = expected.get("line").and_then(toml::Value::as_integer);
    let text = scenario
        .get("given")?
        .get("files")?
        .get(&path)?
        .as_str()?
        .to_owned();
    Some((path, line, text))
}

#[test]
fn check_reports_the_created_rule_where_its_scenario_says() {
    over_samples("check", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        created(&format!("rule/{RULE}"), sample, project)?;
        let (path, line, text) = failing_case(&created_rule(project)?)
            .ok_or("the created rule has no failing scenario")?;
        ran(&["add", RULE, "--at", "."], project)?;
        let written = project.join(&path);
        written
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&written, text))
            .map_err(|error| format!("{path} cannot be written: {error}"))?;
        ran(&["validate"], project)?;
        let found = reported(project)?;
        let expected = vec![(format!("local/{RULE}"), path, line)];
        if found == expected {
            Ok(())
        } else {
            Err(format!("tm check reports {found:?}, not {expected:?}"))
        }
    });
}

#[test]
fn a_rule_entry_that_reaches_no_file_is_a_coverage_error() {
    over_samples("check coverage", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        created(&format!("rule/{RULE}"), sample, project)?;
        ran(&["add", RULE, "--at", "."], project)?;
        let workspace = project.join("tastemaker.toml");
        let text = std::fs::read_to_string(&workspace)
            .map_err(|error| format!("tastemaker.toml cannot be read: {error}"))?;
        let entry = format!("rule = \"local/{RULE}\", include = [\".\"]");
        if !text.contains(&entry) {
            return Err(format!("tm add wrote no `{entry}`:\n{text}"));
        }
        std::fs::write(
            &workspace,
            text.replace(
                &entry,
                &format!("rule = \"local/{RULE}\", include = [\"nowhere/**\"]"),
            ),
        )
        .map_err(|error| format!("tastemaker.toml cannot be written: {error}"))?;
        ran(&["validate"], project)?;
        let found = reported(project)?;
        let coverage = format!("local/{RULE}#coverage");
        if found
            .iter()
            .any(|(rule, path, _)| *rule == coverage && path == "tastemaker.toml")
        {
            Ok(())
        } else {
            Err(format!(
                "tm check reports {found:?}, no `{coverage}` on tastemaker.toml"
            ))
        }
    });
}

#[test]
fn a_lock_that_does_not_read_fails_the_check_and_is_never_rewritten() {
    over_samples("check corrupt lock", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        let lock = project.join("tastemaker.lock");
        let corrupt = "version = [\n";
        std::fs::write(&lock, corrupt)
            .map_err(|error| format!("tastemaker.lock cannot be written: {error}"))?;
        if ran(&["check"], project).is_ok() {
            return Err("tm check passes over a lock that does not read".to_owned());
        }
        if ran(&["check", "--fix"], project).is_ok() {
            return Err("tm check --fix passes over a lock that does not read".to_owned());
        }
        let after = std::fs::read_to_string(&lock)
            .map_err(|error| format!("tastemaker.lock cannot be read: {error}"))?;
        if after == corrupt {
            Ok(())
        } else {
            Err(format!("tm check --fix rewrote the lock:\n{after}"))
        }
    });
}

#[test]
fn check_proves_what_it_uses_with_no_validate_before_it() {
    over_samples("check without validate", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        created(&format!("rule/{RULE}"), sample, project)?;
        let (path, line, text) = failing_case(&created_rule(project)?)
            .ok_or("the created rule has no failing scenario")?;
        ran(&["add", RULE, "--at", "."], project)?;
        let written = project.join(&path);
        written
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&written, text))
            .map_err(|error| format!("{path} cannot be written: {error}"))?;
        let found = reported(project)?;
        let expected = vec![(format!("local/{RULE}"), path, line)];
        if found == expected {
            Ok(())
        } else {
            Err(format!(
                "tm check with no tm validate before it reports {found:?}, not {expected:?}"
            ))
        }
    });
}

#[test]
fn a_definition_whose_own_scenario_fails_fails_the_check() {
    over_samples("check failing definition", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        created(&format!("rule/{RULE}"), sample, project)?;
        ran(&["add", RULE, "--at", "."], project)?;
        let file = project.join(format!("tastemaker/rules/{RULE}/0.1.0/rule.toml"));
        let (path, _, _) = failing_case(&created_rule(project)?)
            .ok_or("the created rule has no failing scenario")?;
        let text = std::fs::read_to_string(&file)
            .map_err(|error| format!("the rule cannot be read: {error}"))?;
        let expectation = format!("path = \"{path}\"");
        if !text.contains(&expectation) {
            return Err(format!(
                "the created rule names no `{expectation}`:\n{text}"
            ));
        }
        std::fs::write(&file, text.replace(&expectation, "path = \"nowhere.txt\""))
            .map_err(|error| format!("the rule cannot be written: {error}"))?;
        let scenario = created_rule(project)?
            .get("scenarios")
            .and_then(toml::Value::as_array)
            .and_then(|scenarios| {
                scenarios.iter().find(|scenario| {
                    scenario
                        .get("then")
                        .and_then(|then| then.get("findings"))
                        .and_then(toml::Value::as_array)
                        .is_some_and(|findings| !findings.is_empty())
                })
            })
            .and_then(|scenario| scenario.get("id"))
            .and_then(toml::Value::as_str)
            .ok_or("the created rule has no failing scenario id")?
            .to_owned();
        // A definition that does not hold stops the check before it lints: it ends as an error (2), in every format, and
        // says why on stderr, so stdout holds nothing another format's reader would take for its output.
        let line = format!(
            "error: `rule local/{RULE}@0.1.0` has no current successful proof (scenario `{scenario}`: expected nowhere.txt"
        );
        for format in ["terminal", "json", "markdown", "sarif"] {
            let (code, printed, said) = ended(&["check", "--format", format], project);
            if code != Some(2_i32)
                || !printed.is_empty()
                || !said.lines().any(|each| each.starts_with(&line))
            {
                return Err(format!(
                    "tm check --format {format} over a rule whose scenario fails ends {code:?}, prints on stdout\n{printed}\nand on stderr no `{line}`:\n{said}"
                ));
            }
        }
        Ok(())
    });
}

/// How many scenario projects a `tm check` checked, as its `--timings=tree` report counts them: one `govern` for each
/// project governed, the workspace's own and each scenario's; none when the check was answered from its kept report. A
/// sample plants findings, so the check must end with the code of a check that found them, and its report must be whole
/// (its header and its column row): a check that failed, or printed no report, is an error, never zero work.
fn governed(project: &std::path::Path) -> Result<usize, String> {
    let checked = tm(&["check", "--timings=tree"], project);
    let report = String::from_utf8_lossy(&checked.stderr).into_owned();
    if checked.status.code() != Some(FOUND) {
        return Err(format!(
            "tm check over a sample exited {:?}, not {FOUND}:\n{report}",
            checked.status.code()
        ));
    }
    if !report.lines().any(|line| line.starts_with(TIMINGS_HEAD))
        || !report.lines().any(|line| line.starts_with(TIMINGS_COLUMNS))
    {
        return Err(format!(
            "tm check printed no whole timings report:\n{report}"
        ));
    }
    report
        .lines()
        .find_map(|line| {
            let mut words = line.split_whitespace();
            (words.next() == Some("govern")).then(|| words.next().map(str::parse::<usize>))
        })
        .flatten()
        .map_or(Ok(0), |calls| {
            calls.map_err(|error| format!("the govern row does not count: {error}\n{report}"))
        })
}

/// The first words of a timings report's head line.
const TIMINGS_HEAD: &str = "WALL attribution";

/// The first words of its column row.
const TIMINGS_COLUMNS: &str = "crate / hex / method";

/// The scenarios of each definition file of the package under `directory`, by its path, in order of path.
fn scenarios(directory: &std::path::Path) -> Result<Vec<(std::path::PathBuf, usize)>, String> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries = std::fs::read_dir(&next)
            .map_err(|error| format!("{} cannot be listed: {error}", next.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|error| format!("{} cannot be listed: {error}", next.display()))?
                .path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path
                .extension()
                .is_some_and(|extension| extension == "toml")
            {
                let table = std::fs::read_to_string(&path)
                    .map_err(|error| format!("{} cannot be read: {error}", path.display()))?
                    .parse::<toml::Table>()
                    .map_err(|error| format!("{} does not parse: {error}", path.display()))?;
                let count = table
                    .get("scenarios")
                    .and_then(toml::Value::as_array)
                    .map_or(0, Vec::len);
                if count > 0 {
                    found.push((path, count));
                }
            }
        }
    }
    found.sort();
    Ok(found)
}

#[test]
fn check_proves_only_what_changed_since_its_last_run() {
    over_samples("check proof cache", |sample| {
        let scratch = copied(sample);
        let project = scratch.path();
        let definitions = scenarios(&project.join("tastemaker"))?;
        let every: usize = definitions.iter().map(|(_, count)| count).sum();
        // The workspace's own check is governed with the scenarios: one more than they are.
        let cold = governed(project)?;
        if cold != every + 1 {
            return Err(format!(
                "a cold tm check governs {cold} projects, not the workspace and its {every} scenarios"
            ));
        }
        let warm = governed(project)?;
        if warm != 0 {
            return Err(format!(
                "a second tm check with nothing changed governs {warm} projects, not none"
            ));
        }
        let (rule, count) = definitions
            .iter()
            .find(|(path, _)| path.ends_with("rule.toml"))
            .ok_or("the sample's package holds no rule with scenarios")?;
        let text = std::fs::read_to_string(rule)
            .map_err(|error| format!("{} cannot be read: {error}", rule.display()))?;
        let edited = text.replacen("description = \"", "description = \"Edited. ", 1);
        if edited == text {
            return Err(format!("{} has no description to edit", rule.display()));
        }
        std::fs::write(rule, edited)
            .map_err(|error| format!("{} cannot be written: {error}", rule.display()))?;
        let after = governed(project)?;
        if after == count + 1 {
            Ok(())
        } else {
            Err(format!(
                "after {} changed, tm check governs {after} projects, not the workspace and that rule's {count} scenarios",
                rule.display()
            ))
        }
    });
}

/// The most resident memory `tm check` may reach over `crates/ryo`, cold: a regression guard at 1.3 GB, set above the 1.21 GB it
/// measured. The goal is 500 MB, and this bound is lowered when the run reaches it.
#[cfg(target_os = "linux")]
const RYO_PEAK_BOUND_KIB: u64 = 1300 * 1024;

/// `tm check` over a `git archive` copy of the repository, in `crates/ryo`, cold, stays under the peak resident set above.
#[test]
#[cfg(target_os = "linux")]
fn checking_crates_ryo_cold_stays_under_its_memory_guard() {
    let scratch = tempfile::tempdir().expect("the scene directory");
    let archive = std::process::Command::new("sh")
        .arg("-c")
        .arg("git -C \"$1\" archive HEAD | tar -x -C \"$2\"")
        .arg("sh")
        .arg(crate::fixes::root())
        .arg(scratch.path())
        .status()
        .expect("git archive starts");
    assert!(archive.success(), "the repository is archived");
    crate::fixes::pinned(&scratch.path().join("crates/ryo/tastemaker.toml"))
        .expect("the workspace pinned to the built tm");
    let init = std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(scratch.path())
        .status()
        .expect("git starts");
    assert!(init.success(), "git init succeeds");
    let output = std::process::Command::new("/usr/bin/time")
        .args(["-f", "peak-kib %M", env!("CARGO_BIN_EXE_tm"), "check"])
        .current_dir(scratch.path().join("crates/ryo"))
        .output()
        .expect("tm starts");
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    // The check ends with its findings (1 or 2) or none (0); anything else is a crash, and says nothing of memory.
    assert!(
        matches!(output.status.code(), Some(0..=2)),
        "tm check ended badly:\n{said}"
    );
    let peak = said
        .lines()
        .find_map(|line| line.strip_prefix("peak-kib "))
        .and_then(|kib| kib.trim().parse::<u64>().ok())
        .unwrap_or_else(|| panic!("`time` reported no peak:\n{said}"));
    assert!(
        peak <= RYO_PEAK_BOUND_KIB,
        "tm check over crates/ryo reached {peak} KiB, over the guard of {RYO_PEAK_BOUND_KIB} KiB"
    );
}

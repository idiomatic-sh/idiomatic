//! `relation list`'s workload.

/// `relation list`, run once per round, which must succeed.
pub(crate) fn workload() -> crate::measure::Measured {
    crate::measure::rounds(
        "relation/list",
        || (),
        |()| {
            let line = <ryo_cli::resources::relation::list::command::ListArguments as clap::Args>::augment_args(
                clap::Command::new("list"),
            );
            let arguments = <ryo_cli::resources::relation::list::command::ListArguments as clap::FromArgMatches>::from_arg_matches(
                &line.get_matches_from(["list"]),
            );
            assert!(
                arguments.is_ok(),
                "`relation list` parses with no arguments"
            );
            let Ok(arguments) = arguments else { return };
            let outcome = ryo_cli::resources::relation::list::command::run(
                arguments,
                &crate::measure::quiet_edge("."),
            );
            assert_eq!(
                outcome,
                ryo_cli::outcome::Outcome::Success,
                "`relation list` did not succeed"
            );
        },
    )
}

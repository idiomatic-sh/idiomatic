//! `relation describe`'s workload.

/// `relation describe` of one relation, run once per round, which must succeed.
pub(crate) fn workload() -> crate::measure::Measured {
    crate::measure::rounds(
        "relation/describe",
        || (),
        |()| {
            let line = <ryo_cli::resources::relation::describe::command::DescribeArguments as clap::Args>::augment_args(
                clap::Command::new("describe"),
            );
            let arguments = <ryo_cli::resources::relation::describe::command::DescribeArguments as clap::FromArgMatches>::from_arg_matches(
                &line.get_matches_from(["describe", "function"]),
            );
            assert!(
                arguments.is_ok(),
                "`relation describe` parses with its name"
            );
            let Ok(arguments) = arguments else { return };
            let outcome = ryo_cli::resources::relation::describe::command::run(
                arguments,
                &crate::measure::quiet_edge("."),
            );
            assert_eq!(
                outcome,
                ryo_cli::outcome::CommandOutcome::Success,
                "`relation describe` did not succeed"
            );
        },
    )
}

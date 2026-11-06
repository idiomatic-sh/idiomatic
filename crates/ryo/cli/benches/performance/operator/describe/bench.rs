//! `operator describe`'s workload.

/// `operator describe` of one operator, run once per round, which must succeed.
pub(crate) fn workload() -> crate::measure::Measured {
    crate::measure::rounds(
        "operator/describe",
        || (),
        |()| {
            let line = <ryo_cli::resources::operator::describe::command::DescribeArguments as clap::Args>::augment_args(
                clap::Command::new("describe"),
            );
            let arguments = <ryo_cli::resources::operator::describe::command::DescribeArguments as clap::FromArgMatches>::from_arg_matches(
                &line.get_matches_from(["describe", "select"]),
            );
            assert!(
                arguments.is_ok(),
                "`operator describe` parses with its name"
            );
            let Ok(arguments) = arguments else { return };
            let outcome = ryo_cli::resources::operator::describe::command::run(
                arguments,
                &crate::measure::quiet_edge("."),
            );
            assert_eq!(
                outcome,
                ryo_cli::outcome::Outcome::Success,
                "`operator describe` did not succeed"
            );
        },
    )
}

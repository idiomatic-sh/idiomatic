//! A long answer on a terminal goes through the user's pager, `PAGER` or `less -FRX`, as
//! git, gh and jj page (clig.dev, Output). `PAGER` is the one environment variable a
//! command line reads, the `pager` exception `no-env-reads` names; this module, which
//! starts the pager, is the terminal's boundary.

use std::io::Write as _;

#[cfg(test)]
#[path = "pager_test.rs"]
mod tests;

/// Writes `answer`'s text through the user's pager when stdout is a terminal, styled as
/// stdout would be, and straight to stdout otherwise or when no pager starts. A pager
/// the user quit early took what they wanted.
pub(super) fn page(answer: &impl std::fmt::Display) -> std::io::Result<()> {
    use std::io::IsTerminal as _;
    let stdout = std::io::stdout();
    let pager = match stdout
        .is_terminal()
        .then(|| pager(std::env::var_os("PAGER")))
        .flatten()
    {
        Some(command) => spawned(command)?,
        None => None,
    };
    let Some(mut pager) = pager else {
        let mut locked = anstream::stdout().lock();
        write!(locked, "{answer}")?;
        return locked.flush();
    };
    let choice = anstream::AutoStream::choice(&stdout);
    let written = pager.stdin.take().map_or(Ok(()), |mut pipe| {
        let sink: &mut dyn std::io::Write = &mut pipe;
        let mut styled = anstream::AutoStream::new(sink, choice);
        write!(styled, "{answer}").and_then(|()| styled.flush())
    });
    // The pager's own exit status is the user's business (they may quit it any way they like); only waiting matters.
    let _: std::process::ExitStatus = pager.wait()?;
    match written {
        Err(failed) if failed.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

/// Streams what `body` writes through the user's pager when stdout is a terminal, and straight to
/// stdout otherwise or when no pager starts, so the text is never held whole: it reaches the
/// reader as it is made. The body is given the stream and owns it: when the body ends the stream
/// is closed. A pager the user quit early took what they wanted.
pub(super) fn page_with(body: impl FnOnce(Box<dyn std::io::Write + Send>)) -> std::io::Result<()> {
    use std::io::IsTerminal as _;
    let stdout = std::io::stdout();
    let pager = match stdout
        .is_terminal()
        .then(|| pager(std::env::var_os("PAGER")))
        .flatten()
    {
        Some(command) => spawned(command)?,
        None => None,
    };
    let Some(mut pager) = pager else {
        body(Box::new(std::io::BufWriter::new(anstream::stdout())));
        return Ok(());
    };
    let choice = anstream::AutoStream::choice(&stdout);
    if let Some(input) = pager.stdin.take() {
        let input: Box<dyn std::io::Write + Send> = Box::new(input);
        body(Box::new(std::io::BufWriter::new(
            anstream::AutoStream::new(input, choice),
        )));
    }
    // The pager's own exit status is the user's business (they may quit it any way they like); only waiting matters.
    let _: std::process::ExitStatus = pager.wait()?;
    Ok(())
}

/// The pager `variable` (`PAGER`) names, split on whitespace (`less -R`), or `less -FRX`
/// when it is unset: quit when the answer fits one screen, keep its styles, leave it on
/// the screen. An empty `PAGER`, or `cat`, asks for no pager, as git takes it.
fn pager(variable: Option<std::ffi::OsString>) -> Option<std::process::Command> {
    let Some(variable) = variable else {
        let mut less = std::process::Command::new("less");
        let _: &mut std::process::Command = less.arg("-FRX");
        return Some(less);
    };
    let variable = variable.to_string_lossy();
    let mut words = variable.split_whitespace();
    let program = words.next().filter(|program| *program != "cat")?;
    let mut pager = std::process::Command::new(program);
    let _: &mut std::process::Command = pager.args(words);
    Some(pager)
}

/// The pager `command` started with its input piped; none when its program is not installed, and the answer goes
/// straight to stdout. Any other failure to start it is the command's.
fn spawned(mut command: std::process::Command) -> std::io::Result<Option<std::process::Child>> {
    match command.stdin(std::process::Stdio::piped()).spawn() {
        Ok(child) => Ok(Some(child)),
        Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(failed) => Err(failed),
    }
}

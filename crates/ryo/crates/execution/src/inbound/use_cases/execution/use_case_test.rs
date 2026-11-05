//! What the `execution` use case promises, against the fakes of the ports it calls, from its fixtures.

use crate::cases::{Case, Word, run};

type Input = ();
type Value = ();
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[];

/// Calls the part on one input; the author states it.
fn subject(_input: Input) -> Result<Value, Failure> {
    todo!("call the part on one input")
}

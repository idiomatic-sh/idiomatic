//! Generated Python sources of a size asked for, in several shapes: what the growth harness and the fillers' bench hold the
//! language's fillers to. A source is one piece of text written again and again, each time with its own names, so its elements
//! (nodes and tokens) grow in step with the pieces and a filler that pairs the names of a file, or reads a whole scope for each
//! of its names, shows at four times the size.

use ryo::languages::Count;

/// The shape of a generated source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrownShape {
    /// Statements one after another at the top of the module, of every construct a relation reads: imports, literals of every
    /// kind, the three ways to format a string, a string read as a type, calls, a decorated function with every kind of
    /// parameter, a class with a base, a name bound twice and a name deleted.
    Statements,
    /// Scopes in scopes: a class holding a method holding a function holding a lambda and a comprehension, each reading the
    /// names of the scopes around it.
    Scopes,
    /// One expression as long as the file: a list display whose items are operations, calls, subscripts and conditionals.
    Expression,
    /// Many names in one scope: one function that binds, binds again and reads name after name.
    Names,
    /// Statements the parser refuses, one on each line.
    Errors,
}

/// Every shape.
pub const SHAPES: [GrownShape; 5] = [
    GrownShape::Statements,
    GrownShape::Scopes,
    GrownShape::Expression,
    GrownShape::Names,
    GrownShape::Errors,
];

/// What stands for a piece's number in its text.
const NUMBER: &str = "{n}";

/// One piece of the `Statements` shape.
const STATEMENTS: &str = "import os.path as path_{n}
from collections import OrderedDict as ordered_{n}
value_{n} = {n} + 1.5  # number {n}
value_{n} = [value_{n}, b\"bytes\", \"text %s and %5.2f\" % (value_{n}, {n}), \"{0} or {name:>4}\".format({n}, name=value_{n})]
label_{n}: \"list[int]\" = [f\"value {value_{n}!r:>8} told\", path_{n}.join(\"a\", \"b\"), len(value_{n}), {1: \"one\", 1.0: \"uno\", (1, 2): None}]
@staticmethod
def function_{n}(first, /, second=2, *rest, third, **more) -> \"int\":
    return first if second else third
class Class_{n}(ordered_{n}, metaclass=type):
    pass
del label_{n}
";

/// One piece of the `Scopes` shape.
const SCOPES: &str = "class Outer_{n}:
    limit_{n} = {n}
    def method_{n}(self, items_{n}):
        def inner_{n}(item):
            nonlocal items_{n}
            return (lambda extra: item + extra + self.limit_{n})([each for each in items_{n} if each])
        return {key: inner_{n}(key) for key in items_{n}}
";

/// How the `Expression` shape begins and ends: every piece is an item of the one list.
const EXPRESSION_ENDS: (&str, &str) = ("values = [\n", "]\n");

/// One piece of the `Expression` shape.
const EXPRESSION: &str = "    (value_{n} := {n}) + other_{n} * 2 if {n} else call_{n}(argument_{n}, key={n})[\"k\"].attribute,\n";

/// How the `Names` shape begins: every piece is in the one function.
const NAMES_START: &str = "def many():\n";

/// One piece of the `Names` shape.
const NAMES: &str = "    name_{n} = {n}\n    name_{n} = name_{n} + 1\n    print(name_{n})\n";

/// One piece of the `Errors` shape.
const ERRORS: &str = "value_{n} = other_{n} +\n";

/// How a shape is written: how it begins, its piece, how it ends, and how many elements one piece adds to a file.
struct Written {
    /// What the source begins with.
    start: &'static str,
    /// The piece written again and again.
    piece: &'static str,
    /// What the source ends with.
    end: &'static str,
    /// The elements one piece adds: its nodes and its tokens, as the language's walk counts them. Read from a walk of the
    /// shape; the contract's case of the sizes holds it.
    elements: usize,
}

/// How the shape `shape` is written.
const fn written(shape: GrownShape) -> Written {
    match shape {
        GrownShape::Statements => Written {
            start: "",
            piece: STATEMENTS,
            end: "",
            elements: 320,
        },
        GrownShape::Scopes => Written {
            start: "",
            piece: SCOPES,
            end: "",
            elements: 154,
        },
        GrownShape::Expression => Written {
            start: EXPRESSION_ENDS.0,
            piece: EXPRESSION,
            end: EXPRESSION_ENDS.1,
            elements: 60,
        },
        GrownShape::Names => Written {
            start: NAMES_START,
            piece: NAMES,
            end: "",
            elements: 37,
        },
        GrownShape::Errors => Written {
            start: "",
            piece: ERRORS,
            end: "",
            elements: 13,
        },
    }
}

/// How many pieces a source of the shape `shape` and of about `elements` elements holds: at least one.
#[must_use]
pub fn pieces(shape: GrownShape, elements: Count) -> Count {
    Count((elements.0 / written(shape).elements).max(1))
}

/// A Python source of the shape `shape` with about `elements` elements (nodes and tokens, as the language's walk counts them):
/// its piece written `pieces(shape, elements)` times, each with its own number in its names.
#[must_use]
pub fn grown_source(shape: GrownShape, elements: Count) -> String {
    let form = written(shape);
    let mut text = String::from(form.start);
    for number in 0..pieces(shape, elements).0 {
        text.push_str(&form.piece.replace(NUMBER, &number.to_string()));
    }
    text.push_str(form.end);
    text
}

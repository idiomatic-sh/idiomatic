//! The allocation rows, measured: the blocks the kernels' programs over `Defines` allocate for a thousand rows and a hundred
//! thousand (two selects sharing one index of its name, the nodes that drop repeated rows through `hash_index`'s keyed rows,
//! and the span kinds of `text_map`, which write one row per row into cells reserved once), a seeded closure for a hundred
//! chains and ten thousand, and the graph kernels over a file's edges, cold and warm.

use super::graph_workloads::{SeededClosure, graph_kernel_blocks, seeded_held};
use super::kernel_programs::{Kernels, kernels_held};

/// The workloads the allocation rows are proved on, within their bounds or past them.
#[derive(Clone, Copy, Debug)]
pub struct Workloads {
    /// Whether each workload is the one past its bound: kernel programs whose answer grows with the rows, the join over a
    /// whole closure, and the graph kernels cold.
    pub past_bound: bool,
}

/// Whether the rows hold for `workloads`. Within the bounds, every row holds: the keyed selects and the seeded closure
/// allocate the same blocks for few rows and many, and the warm graph kernels none. Past them, it holds when any workload
/// past its bound holds its row, so the allocation test's second assertion shows every row able to fail. Every workload
/// runs to the end before any is judged.
///
/// # Panics
///
/// When a workload faults: a program that does not run proves no row, and is no budget breach.
#[must_use]
pub fn held(workloads: &Workloads) -> bool {
    let past = workloads.past_bound;
    let kernels = kernels_held(&Kernels { answering: past });
    let closure = seeded_held(&SeededClosure { fused: !past });
    let graph = {
        let [cold, warm] = graph_kernel_blocks();
        if past { cold == 0 } else { warm == 0 }
    };
    if past {
        kernels || closure || graph
    } else {
        kernels && closure && graph
    }
}

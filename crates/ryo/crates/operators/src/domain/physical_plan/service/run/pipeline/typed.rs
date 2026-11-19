//! Spike (ryo-vhuy): typed chains. A chain of filters and projections whose comparisons the column kinds decide (a symbol
//! column against a text, a number or node column against a number, two node or two number columns) runs as one loop over
//! the head's rows: pass one writes a selection vector of the head rows every filter keeps, comparing each symbol once per
//! run (its verdict memoised by the symbol's id); pass two writes only the tail's cells. Nothing is written between stages.
//! Which nodes may be stages is decided at plan time from their lowering and their input's column kinds (`eligibility`);
//! a chain with any other stage, or a tree head, or a cell its kind does not decide, runs today's path, counted by reason.

pub(crate) mod service;

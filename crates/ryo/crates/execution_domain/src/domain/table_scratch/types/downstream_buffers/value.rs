//! The output sink's downstream buffers, owned together and lent together.

use crate::domain::operating::types::Memo;
use crate::domain::table_scratch::types::OperatorScratch;

/// The buffers a downstream emit borrows into its `OperatorContext`: the output sink's operator scratch and its memo, kept
/// with their room between lends so a delivered row allocates nothing.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::table_scratch::types::downstream_buffers::value::DownstreamBuffers;
/// let mut buffers = DownstreamBuffers::default();
/// buffers.lend().1.reserve(8);
/// let (_, memo) = buffers.lend();
/// assert!(memo.capacity() >= 8);
/// ```
#[derive(Debug, Default)]
pub struct DownstreamBuffers {
    scratch: OperatorScratch,
    memo: Vec<Memo>,
}

impl DownstreamBuffers {
    /// Lends both buffers at once, for one downstream `OperatorContext`. Their room is kept between lends.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::table_scratch::types::downstream_buffers::value::DownstreamBuffers;
    /// let mut buffers = DownstreamBuffers::default();
    /// let (_scratch, memo) = buffers.lend();
    /// assert!(memo.is_empty());
    /// ```
    pub fn lend(&mut self) -> (&mut OperatorScratch, &mut Vec<Memo>) {
        (&mut self.scratch, &mut self.memo)
    }
}

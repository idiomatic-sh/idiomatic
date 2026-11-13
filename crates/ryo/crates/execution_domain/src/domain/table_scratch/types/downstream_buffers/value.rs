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
/// buffers.lend(|_, memo| memo.reserve(8));
/// assert!(buffers.lend(|_, memo| memo.capacity()) >= 8);
/// ```
#[derive(Debug, Default)]
pub struct DownstreamBuffers {
    scratch: OperatorScratch,
    memo: Vec<Memo>,
}

impl DownstreamBuffers {
    /// Lends both buffers at once to `borrow`, for one downstream `OperatorContext`, and answers what it answers. Their room
    /// is kept between lends.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::table_scratch::types::downstream_buffers::value::DownstreamBuffers;
    /// let mut buffers = DownstreamBuffers::default();
    /// assert!(buffers.lend(|_scratch, memo| memo.is_empty()));
    /// ```
    pub fn lend<R>(&mut self, borrow: impl FnOnce(&mut OperatorScratch, &mut Vec<Memo>) -> R) -> R {
        borrow(&mut self.scratch, &mut self.memo)
    }
}

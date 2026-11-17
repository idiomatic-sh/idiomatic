//! A file of the postcard store adapter's test support: the staging append and read over any standard-IO file, so failure cases
//! need no operating-system permissions.

use crate::outbound::lint_store::adapters::postcard::connection::invalid;
use crate::outbound::lint_store::adapters::postcard::connection::validate;
use crate::outbound::lint_store::adapters::postcard::constants::STAGED_OFFSET_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::STAGED_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY;
use crate::outbound::lint_store::adapters::postcard::constants::STAGED_PAYLOAD_LENGTH_OVERFLOWS;
use ryo_values::domain::records::types::PayloadLength;
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::records::types::StagingSession;
use ryo_values::domain::stored_facts::types::PackedWords;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

/// Standard-IO seams keep failure tests independent of operating-system permissions.
pub(crate) fn append<F: Write + Seek>(
    file: &mut F,
    session: StagingSession,
    held: &mut PayloadLength,
    words: &PackedWords,
) -> std::io::Result<StagedWords> {
    let length =
        u64::try_from(words.0.len()).map_err(|_| invalid(STAGED_PAYLOAD_LENGTH_OVERFLOWS))?;
    let end = held
        .0
        .checked_add(length)
        .ok_or_else(|| invalid(STAGED_OFFSET_OVERFLOWS))?;
    let _position = file.seek(SeekFrom::Start(held.0))?;
    file.write_all(&words.0)?;
    // Commit logical length only after every append/flush operation succeeds.
    file.flush()?;
    let span = StagedWords {
        session,
        offset: ryo_values::domain::records::types::PayloadOffset(held.0),
        length: ryo_values::domain::records::types::PayloadLength(length),
    };
    *held = PayloadLength(end);
    Ok(span)
}

/// At most one selected normalized bucket is owned by the replay caller.
pub(crate) fn read<F: Read + Write + Seek>(
    file: &mut F,
    session: StagingSession,
    held: PayloadLength,
    span: &StagedWords,
    into: &mut PackedWords,
) -> std::io::Result<()> {
    validate(session, held, span)?;
    let length = usize::try_from(span.length.0)
        .map_err(|_| invalid(STAGED_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY))?;
    file.flush()?;
    let _position = file.seek(SeekFrom::Start(span.offset.0))?;
    into.0.clear();
    into.0
        .try_reserve_exact(length)
        .map_err(std::io::Error::other)?;
    into.0.resize(length, 0);
    file.read_exact(&mut into.0)
}

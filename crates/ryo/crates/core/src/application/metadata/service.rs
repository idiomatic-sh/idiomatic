//! The decisions over request metadata the use cases take: the kernel's own, called here where an application service may name its
//! implementation, so a use case holds none.

use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;

/// The kernel's decisions over request metadata, for a use case to call.
#[must_use]
pub fn kernel_metadata() -> impl MetadataUseCase {
    MetadataUseCaseImpl
}

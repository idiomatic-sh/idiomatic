// planted: Consistent file names (no use_case.rs here)
pub struct ArchiveTodoUseCase;

impl ArchiveTodoUseCase {
    pub async fn run(&self, id: u64) -> u64 {
        id
    }
}

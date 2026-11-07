use super::service::exact_required_files;
use super::types::FileAccounting;
use ryo_relations::domain::relations::types::FileId;

#[test]
fn a_complete_bucket_requires_every_declared_project_file() {
    let required = [FileId(0), FileId(1)];
    assert_eq!(
        exact_required_files(&required, &[FileId(0)].into()),
        FileAccounting::Inexact
    );
    assert_eq!(
        exact_required_files(&required, &[FileId(0), FileId(1)].into()),
        FileAccounting::Exact
    );
    assert_eq!(
        exact_required_files(&required, &[FileId(0), FileId(1), FileId(2)].into()),
        FileAccounting::Inexact
    );
}

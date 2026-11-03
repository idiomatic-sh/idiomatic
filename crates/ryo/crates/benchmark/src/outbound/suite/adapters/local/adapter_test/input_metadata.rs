//! Real source staging and per-attempt permission verification in both context modes.
use super::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

fn fixture(root: &std::path::Path, mode: &str, declare: bool) -> std::path::PathBuf {
    let rules = root.join("rules");
    std::fs::create_dir_all(rules.join("A001")).unwrap();
    std::fs::write(root.join("benchmark.toml"), format!("tool='fixture'\nversion='1'\nsha256=['{}']\ndefaults=['A001']\nproject-context-mode='{mode}'\nfile-observation='posix-executable'\n", "a".repeat(64))).unwrap();
    std::fs::write(rules.join("A001/query.ryoql"), "let files = select path from \"filesystem.executable\"; emit files subject path key(path);").unwrap();
    let context = if mode == "listed-disk" {
        "context={logical-root='repo',auxiliary-files={}}\n"
    } else {
        ""
    };
    let metadata = if declare {
        "file-metadata={'case.txt'=true}\n"
    } else {
        ""
    };
    let cases = format!(
        "[[cases]]\nname='positive'\ncomparison='full'\n{context}files={{'case.txt'='data'}}\n{metadata}expected=[{{rule='A001',file='case.txt',start=0,end=0}}]\n[[cases]]\nname='negative'\ncomparison='full'\n{context}files={{'case.txt'='data'}}\n{metadata}expected=[]\n"
    );
    std::fs::write(rules.join("A001/cases.toml"), cases).unwrap();
    rules
}

#[cfg(unix)]
#[test]
fn both_materialization_routes_stage_and_reverify_declared_mode_before_each_attempt() {
    for mode in ["isolated", "listed-disk"] {
        let root = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let rules = fixture(root.path(), mode, true);
        let cases = SuiteLocalAdapter
            .cases(ArtifactPath(&rules), ArtifactPath(scratch.path()))
            .unwrap();
        for case in cases {
            SuiteLocalAdapter
                .validate_file_metadata(&case.request)
                .unwrap();
            let path = case.request.root.join("case.txt");
            let before = std::fs::metadata(&path).unwrap();
            std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(before.permissions().mode() & !0o111),
            )
            .unwrap();
            assert!(
                SuiteLocalAdapter
                    .validate_file_metadata(&case.request)
                    .is_err()
            );
            assert_eq!(std::fs::read(&path).unwrap(), b"data");
            assert_eq!(
                std::fs::metadata(&path).unwrap().modified().unwrap(),
                before.modified().unwrap()
            );
            std::fs::set_permissions(&path, before.permissions()).unwrap();
            SuiteLocalAdapter
                .validate_file_metadata(&case.request)
                .unwrap();
            std::fs::remove_file(&path).unwrap();
            assert!(
                SuiteLocalAdapter
                    .validate_file_metadata(&case.request)
                    .is_err()
            );
        }
    }
}

#[test]
fn required_profile_cannot_admit_omitted_observations_even_for_negative_cases() {
    let root = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let rules = fixture(root.path(), "isolated", false);
    assert!(
        SuiteLocalAdapter
            .cases(ArtifactPath(&rules), ArtifactPath(scratch.path()))
            .is_err()
    );
}

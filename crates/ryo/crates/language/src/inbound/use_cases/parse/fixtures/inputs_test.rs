//! The rails' read of a fixtures folder: every independent input of a relation is held to the rails.

use super::super::findings::Findings;
use super::{Category, inputs_in};

/// The same independent files used for typed rows must also pass syntax/shape/oracle rails.
#[test]
fn relation_rails_include_each_independent_input() -> std::io::Result<()> {
    let root = std::env::temp_dir().join(format!("ryo-multiinput-rails-{}", std::process::id()));
    let folder = root.join("relations").join("sample");
    std::fs::create_dir_all(&folder)?;
    std::fs::write(folder.join("input.py"), "first")?;
    std::fs::write(folder.join("input.other.py"), "second")?;
    let mut findings = Findings(Vec::new());
    let actual: Vec<_> = inputs_in(&root, &mut findings)
        .into_iter()
        .filter(|input| input.category == Category::Fixture)
        .map(|input| input.shown)
        .collect();
    assert_eq!(
        actual,
        ["relations/sample/other.py", "relations/sample/input.py"]
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}

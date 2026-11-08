use ryo_execution_runtime::application::file_dag::frontiers::types::PreparedFrontiers;
fn forge() {
    let _ = PreparedFrontiers {
        rows: vec![],
        refused: vec![],
        program: panic!("not caller-owned"),
        required: panic!("not caller-owned"),
    };
}
fn main() {}

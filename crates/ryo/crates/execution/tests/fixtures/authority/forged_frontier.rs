use ryo_execution::inbound::use_cases::execution::types::PreparedFrontiers;
fn forge() {
    let _ = PreparedFrontiers { rows: vec![], refused: vec![], program: panic!("not caller-owned"), required: panic!("not caller-owned") };
}
fn main() {}

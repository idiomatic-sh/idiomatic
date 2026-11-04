use super::*;
use crate::domain::operating::types::Memo;
use ryo_relations::domain::relations::types::{Integer, Value};
#[test]
fn nested_different_node_demand_keeps_outer_scratch_and_releases_both_on_error() {
    let rooms = InvocationRooms::new(2);
    let result = (|| -> Result<(), OperatingError> {
        let mut outer = rooms.take(NodeId(0))?;
        outer.room()?.scratch.row.push(Value::Number(Integer(654)));
        outer.room()?.memo.push(Memo::Fails);
        {
            let mut nested = rooms.take(NodeId(1))?;
            nested.room()?.scratch.row.push(Value::Number(Integer(987)));
            assert!(
                rooms.take(NodeId(0)).is_err(),
                "same-node recursion is refused"
            );
            assert_eq!(outer.room()?.scratch.row, [Value::Number(Integer(654))]);
            assert!(matches!(outer.room()?.memo.as_slice(), [Memo::Fails]));
        }
        Err(OperatingError::WrongBinding)
    })();
    assert!(matches!(result, Err(OperatingError::WrongBinding)));
    let mut outer = rooms.take(NodeId(0)).unwrap();
    let mut nested = rooms.take(NodeId(1)).unwrap();
    assert_eq!(
        outer.room().unwrap().scratch.row,
        [Value::Number(Integer(654))]
    );
    assert_eq!(
        nested.room().unwrap().scratch.row,
        [Value::Number(Integer(987))]
    );
    assert!(matches!(
        outer.room().unwrap().memo.as_slice(),
        [Memo::Fails]
    ));
}

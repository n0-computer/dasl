use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct TupleStruct(String, i32, u64);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct UnitStruct;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Struct<'a> {
    tuple_struct: TupleStruct,
    tuple: (String, f32, f64),
    map: BTreeMap<String, String>,
    bytes: &'a [u8],
    array: Vec<String>,
}

use std::iter::FromIterator;

#[test]
fn serde() {
    let tuple_struct = TupleStruct("test".to_string(), -60, 3000);

    let tuple = ("hello".to_string(), -50.004097, -12.094635556478);

    let map = BTreeMap::from_iter(
        [
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
            ("key3".to_string(), "value3".to_string()),
            ("key4".to_string(), "value4".to_string()),
        ]
        .iter()
        .cloned(),
    );

    let bytes = b"test byte string";

    let array = vec!["one".to_string(), "two".to_string(), "three".to_string()];

    let data = Struct {
        tuple_struct,
        tuple,
        map,
        bytes,
        array,
    };

    let data_ser = dasl::drisl::to_vec(&data).unwrap();
    let data_back = dasl::drisl::from_slice(&data_ser).unwrap();

    assert_eq!(data, data_back);
}

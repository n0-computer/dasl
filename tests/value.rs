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

#[allow(clippy::useless_format)]
#[test]
fn serde() {
    let tuple_struct = TupleStruct(format!("test"), -60, 3000);

    let tuple = (format!("hello"), -50.004097, -12.094635556478);

    let map = BTreeMap::from_iter(
        [
            (format!("key1"), format!("value1")),
            (format!("key2"), format!("value2")),
            (format!("key3"), format!("value3")),
            (format!("key4"), format!("value4")),
        ]
        .iter()
        .cloned(),
    );

    let bytes = b"test byte string";

    let array = vec![format!("one"), format!("two"), format!("three")];

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

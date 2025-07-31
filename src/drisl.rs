//! DRISL serialization and deserialization.
//!
//! Implementation originally based on [`serde_ipld_dagcbor`](https://github.com/ipld/serde_ipld_dagcbor)
//! and parts of [`cbor4ii`](https://docs.rs/cbor4ii).

mod cbor4ii_nonpub;
mod value;

pub mod de;
pub mod error;
pub mod ser;

#[doc(inline)]
pub use value::Value;

#[doc(inline)]
pub use self::de::from_reader;
// Convenience functions for serialization and deserialization.
#[doc(inline)]
pub use self::de::from_slice;
#[doc(inline)]
pub use self::error::{DecodeError, EncodeError};
#[doc(inline)]
pub use self::ser::to_vec;
#[doc(inline)]
pub use self::ser::to_writer;

/// The CBOR tag that is used for CIDs.
const CBOR_TAGS_CID: u8 = 42;

pub use serde_bytes;

#[cfg(test)]
mod tests {
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
        #[serde(with = "serde_bytes")]
        bytes: &'a [u8],
        array: Vec<String>,
    }

    use std::iter::FromIterator;

    #[test]
    fn basics() {
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

        let data_ser = super::to_vec(&data).unwrap();
        println!("{}", hex::encode(&data_ser));
        let data_back = super::from_slice(&data_ser).unwrap();

        assert_eq!(data, data_back);
    }
}

//! CID Serde (de)serialization
//!
//! CIDs cannot directly be represented in any of the native Serde Data model types. In order to
//! work around that limitation. a newtype struct is introduced, that is used as a marker for Serde
//! (de)serialization.
//!
//! Based on https://github.com/multiformats/rust-cid/blob/master/src/serde.rs

use core::fmt;
use std::{format, vec::Vec};

use serde::{de, ser};
use serde_bytes::ByteBuf;

use super::Cid;

/// An identifier that is used internally by Serde implementations that support [`Cid`]s.
// TODO: should this be different than the one in `rust-cid`?
pub const CID_SERDE_PRIVATE_IDENTIFIER: &str = "$__private__serde__identifier__for__cid";

/// Serialize a CID into the Serde data model as enum.
///
/// Custom types are not supported by Serde, hence we map a CID into an enum that can be identified
/// as a CID by implementations that support CIDs. The corresponding Rust type would be:
///
/// ```text
/// struct $__private__serde__identifier__for__cid(serde_bytes::BytesBuf);
/// ```
impl ser::Serialize for Cid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let value = ByteBuf::from(self.as_bytes());
        serializer.serialize_newtype_struct(CID_SERDE_PRIVATE_IDENTIFIER, &value)
    }
}

/// Visitor to transform bytes into a CID.
pub struct BytesToCidVisitor;

impl<'de> de::Visitor<'de> for BytesToCidVisitor {
    type Value = Cid;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "a valid CID in bytes")
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Cid::from_bytes_raw(value)
            .map_err(|err| de::Error::custom(format!("Failed to deserialize CID: {err}")))
    }

    /// Some Serde data formats interpret a byte stream as a sequence of bytes (e.g. `serde_json`).
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut bytes = Vec::new();
        while let Some(byte) = seq.next_element()? {
            bytes.push(byte);
        }
        Cid::from_bytes_raw(&bytes)
            .map_err(|err| de::Error::custom(format!("Failed to deserialize CID: {err}")))
    }
}

/// Deserialize a CID into a newtype struct.
///
/// Deserialize a CID that was serialized as a newtype struct, so that can be identified as a CID.
/// Its corresponding Rust type would be:
///
/// ```text
/// struct $__private__serde__identifier__for__cid(serde_bytes::BytesBuf);
/// ```
impl<'de> de::Deserialize<'de> for Cid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        /// Main visitor to deserialize a CID.
        ///
        /// This visitor has only a single entry point to deserialize CIDs, it's
        /// `visit_new_type_struct()`. This ensures that it isn't accidentally used to decode CIDs
        /// to bytes.
        struct MainEntryVisitor;

        impl<'de> de::Visitor<'de> for MainEntryVisitor {
            type Value = Cid;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "a valid CID in bytes, wrapped in an newtype struct")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                deserializer.deserialize_bytes(BytesToCidVisitor)
            }
        }

        deserializer.deserialize_newtype_struct(CID_SERDE_PRIVATE_IDENTIFIER, MainEntryVisitor)
    }
}

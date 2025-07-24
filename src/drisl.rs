//! DRISL serialization and deserialization.
//!
//! Implementation originally based on [`serde_ipld_dagcbor`](https://github.com/ipld/serde_ipld_dagcbor) and parts of [`cbor4ii`](https://docs.rs/cbor4ii).

mod cbor4ii_nonpub;
pub mod de;
pub mod error;
pub mod ser;

#[doc(inline)]
pub use self::error::{DecodeError, EncodeError};

// Convenience functions for serialization and deserialization.
#[doc(inline)]
pub use self::de::from_slice;

#[doc(inline)]
pub use self::de::from_reader;

#[doc(inline)]
pub use self::ser::to_vec;

#[doc(inline)]
pub use self::ser::to_writer;

/// The CBOR tag that is used for CIDs.
const CBOR_TAGS_CID: u8 = 42;

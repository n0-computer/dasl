//! CIDs (Content IDs) are identifiers used for addressing resources by their contents, essentially a hash with limited metadata.
//!
//! [Spec](https://dasl.ing/cid.html)

use std::{fmt::Display, str::FromStr};

use sha2::Digest;
use thiserror::Error;

use crate::base32::BASE32_LOWER;

mod serde;

pub(crate) use self::serde::{BytesToCidVisitor, CID_SERDE_PRIVATE_IDENTIFIER};

const CID_VERSION: u8 = 1;
const PREFIX_LEN: usize = 4;
/// Length of a known hash
const HASH_LEN: u8 = 32;
const DATA_LEN: usize = PREFIX_LEN + HASH_LEN as usize;
const HASH_CODE_SHA2_256: u8 = 0x12;
const HASH_CODE_BLAKE3: u8 = 0x1e;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct Cid {
    // - 1 byte CID version
    // - 1 byte Codec
    // - 1 byte hash type
    // - 1 byte Length
    // - 32 bytes hash
    data: [u8; DATA_LEN],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[non_exhaustive]
#[repr(u8)]
pub enum Codec {
    Raw = 0x55,
    Drisl = 0x71,
}

#[derive(Debug, Error)]
pub enum ParseCodecError {
    #[error("Unknown codec: {_0:x}")]
    UnknownCodec(u8),
}

impl TryFrom<u8> for Codec {
    type Error = ParseCodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x55 => Ok(Self::Raw),
            0x71 => Ok(Self::Drisl),
            _ => Err(ParseCodecError::UnknownCodec(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[non_exhaustive]
#[repr(u8)]
pub enum Multihash {
    Sha2256 = 0x12,
    Blake3 = 0x1e,
}

impl TryFrom<u8> for Multihash {
    type Error = MultihashParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            HASH_CODE_SHA2_256 => Ok(Self::Sha2256),
            HASH_CODE_BLAKE3 => Ok(Self::Blake3),
            _ => Err(MultihashParseError::UnknownHash(value)),
        }
    }
}

#[derive(Debug, Error)]
pub enum CidParseError {
    #[error("Invalid encoding")]
    InvalidEncoding,
    #[error("Too short")]
    TooShort,
    #[error("Invalid CID version: {_0}")]
    InvalidCidVersion(u8),
    #[error("Invalid codec")]
    InvalidCodec(ParseCodecError),
    #[error("Invalid multihash")]
    InvalidMultihash(MultihashParseError),
}

impl From<ParseCodecError> for CidParseError {
    fn from(err: ParseCodecError) -> Self {
        Self::InvalidCodec(err)
    }
}

impl From<MultihashParseError> for CidParseError {
    fn from(err: MultihashParseError) -> Self {
        Self::InvalidMultihash(err)
    }
}

impl FromStr for Cid {
    type Err = CidParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with('b') {
            return Err(CidParseError::InvalidEncoding);
        }

        // skip base encoding prefix
        let without_prefix = &s.as_bytes()[1..];
        let bytes = BASE32_LOWER
            .decode(without_prefix)
            .map_err(|_e| CidParseError::InvalidEncoding)?;

        Cid::from_bytes_raw(&bytes)
    }
}

impl Cid {
    /// Returns the `Multihash` of this `CID`.
    pub fn hash(&self) -> &[u8] {
        match self.data[3] {
            0 => &[][..], // empty hash
            HASH_LEN => &self.data[PREFIX_LEN..],
            _ => unreachable!("invalid construction"),
        }
    }

    pub fn multihash_type(&self) -> Multihash {
        Multihash::try_from(self.data[2]).expect("invalid construction")
    }

    /// Returns the `Codec` of this `CID`.
    pub fn codec(&self) -> Codec {
        Codec::try_from(self.data[1]).expect("invalid construction")
    }

    /// Tries to decode a `CID` from binary encoding.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CidParseError> {
        dbg!(data_encoding::HEXLOWER.encode(bytes));
        if bytes.is_empty() {
            return Err(CidParseError::TooShort);
        }
        if bytes[0] != 0x0 {
            return Err(CidParseError::InvalidEncoding);
        }
        Self::from_bytes_raw(&bytes[1..])
    }

    /// Tries to decode a `CID` from its raw binary components.
    pub fn from_bytes_raw(bytes: &[u8]) -> Result<Self, CidParseError> {
        dbg!(data_encoding::HEXLOWER.encode(bytes));
        const MIN_LEN: usize = 3;

        if bytes.len() < MIN_LEN {
            return Err(CidParseError::TooShort);
        }
        if bytes.len() > DATA_LEN {
            return Err(MultihashParseError::InvalidLength(bytes.len()).into());
        }

        if bytes[0] != CID_VERSION {
            return Err(CidParseError::InvalidCidVersion(bytes[0]));
        }
        let mut data = [0u8; DATA_LEN];
        let _codec = Codec::try_from(bytes[1])?;
        let _multihash = Multihash::try_from(bytes[2])?;

        let len = bytes[3];
        match len {
            0 => {
                if bytes.len() > 4 {
                    return Err(MultihashParseError::InvalidLength(bytes.len()).into());
                }
                data[..PREFIX_LEN].copy_from_slice(&bytes[..PREFIX_LEN]);
            }
            HASH_LEN => {
                if bytes.len() != DATA_LEN {
                    return Err(MultihashParseError::InvalidLength(bytes.len()).into());
                }
                data.copy_from_slice(bytes);
            }
            _ => return Err(MultihashParseError::InvalidLengthPrefix.into()),
        }

        Ok(Cid { data })
    }

    /// Encode the `CID` in its raw binary format.
    pub fn as_bytes(&self) -> &[u8] {
        match self.data[3] {
            0 => &self.data[..PREFIX_LEN],
            HASH_LEN => &self.data,
            _ => unreachable!("invalid construction"),
        }
    }

    pub fn digest_sha2(codec: Codec, data: impl AsRef<[u8]>) -> Self {
        let hash = sha2::Sha256::digest(data);
        let mut data = [0u8; DATA_LEN];
        data[0] = CID_VERSION;
        data[1] = codec as u8;
        data[2] = HASH_CODE_SHA2_256;
        data[3] = HASH_LEN;
        data[PREFIX_LEN..].copy_from_slice(&hash);
        Self { data }
    }

    pub fn digest_blake3(codec: Codec, data: impl AsRef<[u8]>) -> Self {
        let hash = blake3::hash(data.as_ref());
        let mut data = [0u8; DATA_LEN];
        data[0] = CID_VERSION;
        data[1] = codec as u8;
        data[2] = HASH_CODE_BLAKE3;
        data[3] = HASH_LEN;
        data[PREFIX_LEN..].copy_from_slice(hash.as_bytes());
        Self { data }
    }

    pub fn empty_sha2_256(codec: Codec) -> Self {
        let mut data = [0u8; DATA_LEN];
        data[0] = CID_VERSION;
        data[1] = codec as u8;
        data[2] = HASH_CODE_SHA2_256;
        data[3] = 0;
        Self { data }
    }

    pub fn empty_blake3(codec: Codec) -> Self {
        let mut data = [0u8; DATA_LEN];
        data[0] = CID_VERSION;
        data[1] = codec as u8;
        data[2] = HASH_CODE_BLAKE3;
        data[3] = 0;
        Self { data }
    }
}

impl Display for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b")?;
        let out = self.as_bytes();
        BASE32_LOWER.encode_write(out, f)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MultihashParseError {
    #[error("Invalid length: {_0}")]
    InvalidLength(usize),
    #[error("Unknown hash: {_0:x}")]
    UnknownHash(u8),
    #[error("Invalid length prefix")]
    InvalidLengthPrefix,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_sha2_256() {
        // Sha2 256: "foo"
        let cid_str = "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy";
        let parsed: Cid = cid_str.parse().unwrap();
        assert_eq!(parsed.codec(), Codec::Raw);
        assert!(matches!(parsed.multihash_type(), Multihash::Sha2256));

        let cid_str_back = parsed.to_string();
        assert_eq!(cid_str_back, cid_str);
    }

    #[test]
    fn test_base_blake3() {
        // Blake3: "foo"
        let cid_str = "bafkr4iae4c5tt4yldi76xcpvg3etxykqkvec352im5fqbutolj2xo5yc5e";
        let parsed: Cid = cid_str.parse().unwrap();
        assert_eq!(parsed.codec(), Codec::Raw);
        assert!(matches!(parsed.multihash_type(), Multihash::Blake3));

        let cid_str_back = parsed.to_string();
        assert_eq!(cid_str_back, cid_str);
    }

    #[test]
    fn test_digest_sha2_256() {
        let cid_str = "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy";
        assert_eq!(Cid::digest_sha2(Codec::Raw, b"foo").to_string(), cid_str);
    }

    #[test]
    fn test_digest_blake3() {
        let cid_str = "bafkr4iae4c5tt4yldi76xcpvg3etxykqkvec352im5fqbutolj2xo5yc5e";
        assert_eq!(Cid::digest_blake3(Codec::Raw, b"foo").to_string(), cid_str);
    }
}

//! CIDs (Content IDs) are identifiers used for addressing resources by their contents, essentially a hash with limited metadata.
//!
//! [Spec](https://dasl.ing/cid.html)

use std::{fmt::Display, str::FromStr};

use sha2::Digest;
use thiserror::Error;

use crate::base32::BASE32_LOWER;

mod serde;

pub(crate) use serde::CID_SERDE_PRIVATE_IDENTIFIER;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cid {
    codec: Codec,
    hash: Multihash,
}

const CID_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Multihash {
    Sha2256([u8; 32]),
    Blake3([u8; 32]),
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
    pub fn hash(&self) -> &Multihash {
        &self.hash
    }

    /// Returns the `Codec` of this `CID`.
    pub fn codec(&self) -> Codec {
        self.codec
    }

    /// Tries to decode a `CID` from binary encoding.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CidParseError> {
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
        const MIN_LEN: usize = 3;

        if bytes.len() < MIN_LEN {
            return Err(CidParseError::TooShort);
        }

        if bytes[0] != CID_VERSION {
            return Err(CidParseError::InvalidCidVersion(bytes[0]));
        }

        let codec = Codec::try_from(bytes[1])?;
        let hash = Multihash::from_bytes(&bytes[2..])?;

        Ok(Cid { codec, hash })
    }

    /// Encode the `CID` in binary format.
    pub fn as_bytes(&self) -> [u8; 37] {
        let mut out = [0u8; 37];
        // out[0] = 0 binary prefix
        out[1..].copy_from_slice(&self.as_bytes_raw());
        out
    }

    /// Encodes the `CID` in its raw binary components.
    pub fn as_bytes_raw(&self) -> [u8; 36] {
        let mut out = [0u8; 36];
        out[0] = CID_VERSION;
        out[1] = self.codec as u8;
        out[2..].copy_from_slice(&self.hash.as_bytes());

        out
    }

    pub fn digest_sha2(codec: Codec, data: impl AsRef<[u8]>) -> Self {
        Self {
            codec,
            hash: Multihash::digest_sha2(data),
        }
    }

    pub fn digest_blake3(codec: Codec, data: impl AsRef<[u8]>) -> Self {
        Self {
            codec,
            hash: Multihash::digest_blake3(data),
        }
    }
}

impl Display for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b")?;
        let out = self.as_bytes_raw();
        BASE32_LOWER.encode_write(&out, f)?;

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

/// Length of a known hash
const HASH_LEN: u8 = 32;

const HASH_CODE_SHA2_256: u8 = 0x12;
const HASH_CODE_BLAKE3: u8 = 0x1e;

impl Multihash {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MultihashParseError> {
        if bytes.len() != 1 + 1 + HASH_LEN as usize {
            return Err(MultihashParseError::InvalidLength(bytes.len()));
        }

        match bytes[0] {
            HASH_CODE_SHA2_256 => {
                if bytes[1] != HASH_LEN {
                    return Err(MultihashParseError::InvalidLengthPrefix);
                }
                let hash = bytes[2..].try_into().expect("checked");
                Ok(Self::Sha2256(hash))
            }
            HASH_CODE_BLAKE3 => {
                if bytes[1] != HASH_LEN {
                    return Err(MultihashParseError::InvalidLengthPrefix);
                }
                let hash = bytes[2..].try_into().expect("checked");
                Ok(Self::Blake3(hash))
            }
            _ => Err(MultihashParseError::UnknownHash(bytes[0])),
        }
    }

    pub fn as_bytes(&self) -> [u8; 34] {
        let mut out = [0u8; 34];
        out[1] = HASH_LEN;
        match self {
            Self::Sha2256(hash) => {
                out[0] = HASH_CODE_SHA2_256;
                out[2..].copy_from_slice(hash);
            }
            Self::Blake3(hash) => {
                out[0] = HASH_CODE_BLAKE3;
                out[2..].copy_from_slice(hash);
            }
        }
        out
    }

    pub fn digest_sha2(data: impl AsRef<[u8]>) -> Self {
        let hash = sha2::Sha256::digest(data);
        Self::Sha2256(hash.into())
    }

    pub fn digest_blake3(data: impl AsRef<[u8]>) -> Self {
        let hash = blake3::hash(data.as_ref());
        Self::Blake3(hash.into())
    }
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
        assert!(matches!(parsed.hash(), Multihash::Sha2256(_)));

        let cid_str_back = parsed.to_string();
        assert_eq!(cid_str_back, cid_str);
    }

    #[test]
    fn test_base_blake3() {
        // Blake3: "foo"
        let cid_str = "bafkr4iae4c5tt4yldi76xcpvg3etxykqkvec352im5fqbutolj2xo5yc5e";
        let parsed: Cid = cid_str.parse().unwrap();
        assert_eq!(parsed.codec(), Codec::Raw);
        assert!(matches!(parsed.hash(), Multihash::Blake3(_)));

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

//! CIDs (Content IDs) are identifiers used for addressing resources by their contents, essentially a hash with limited metadata.
//!
//! [Spec](https://dasl.ing/cid.html)

use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cid {
    codec: Codec,
    hash: Multihash,
}

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
        if s.chars().next() != Some('b') {
            dbg!(&s);
            return Err(CidParseError::InvalidEncoding);
        }

        // skip base encoding prefix
        let bytes = data_encoding::BASE32_NOPAD_NOCASE
            .decode(&s[1..].as_bytes())
            .map_err(|_e| CidParseError::InvalidEncoding)?;

        Cid::from_bytes(&bytes)
    }
}

impl Cid {
    pub fn hash(&self) -> &Multihash {
        &self.hash
    }

    pub fn codec(&self) -> Codec {
        self.codec
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CidParseError> {
        const MIN_LEN: usize = 3;
        const CID_VERSION: u8 = 1;

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

impl Multihash {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MultihashParseError> {
        const HASH_LEN: u8 = 32;
        if bytes.len() != 1 + 1 + HASH_LEN as usize {
            return Err(MultihashParseError::InvalidLength(bytes.len()));
        }

        match bytes[0] {
            0x12 => {
                if bytes[1] != HASH_LEN {
                    return Err(MultihashParseError::InvalidLengthPrefix);
                }
                let hash = bytes[2..].try_into().expect("checked");
                Ok(Self::Sha2256(hash))
            }
            0x1e => {
                if bytes[1] != HASH_LEN {
                    return Err(MultihashParseError::InvalidLengthPrefix);
                }
                let hash = bytes[2..].try_into().expect("checked");
                Ok(Self::Blake3(hash))
            }
            _ => Err(MultihashParseError::UnknownHash(bytes[0])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_decode_sha2_256() {
        // Sha2 256: "foo"
        let cid_str = "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy";
        let parsed: Cid = cid_str.parse().unwrap();
        assert_eq!(parsed.codec(), Codec::Raw);
        assert!(matches!(parsed.hash(), Multihash::Sha2256(_)));
    }
}

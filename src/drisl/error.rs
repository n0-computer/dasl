//! When serializing or deserializing DRISL goes wrong.

use core::{convert::Infallible, fmt};
use std::{
    collections::TryReserveError,
    string::{String, ToString},
};

use cbor4ii::core::error::Len;
use serde::{de, ser};

/// An encoding error.
#[derive(Debug)]
pub enum EncodeError<E> {
    /// Custom error message.
    Msg(String),
    /// IO Error.
    Write(E),
}

impl<E> From<E> for EncodeError<E> {
    fn from(err: E) -> EncodeError<E> {
        EncodeError::Write(err)
    }
}

impl<E: core::error::Error + 'static> ser::Error for EncodeError<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        EncodeError::Msg(msg.to_string())
    }
}

impl<E: core::error::Error + 'static> core::error::Error for EncodeError<E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            EncodeError::Msg(_) => None,
            EncodeError::Write(err) => Some(err),
        }
    }
}

impl<E: fmt::Debug> fmt::Display for EncodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<E: fmt::Debug> From<cbor4ii::core::error::EncodeError<E>> for EncodeError<E> {
    fn from(err: cbor4ii::core::error::EncodeError<E>) -> EncodeError<E> {
        match err {
            cbor4ii::core::error::EncodeError::Write(e) => EncodeError::Write(e),
            // Needed as `cbor4ii::core::error::EncodeError` is markes as non_exhaustive
            _ => EncodeError::Msg(err.to_string()),
        }
    }
}

/// A decoding error.
#[derive(Debug)]
pub enum DecodeError<E> {
    /// Custom error message.
    Msg(String),
    /// IO error.
    Read(E),
    /// End of file.
    Eof { name: &'static str, expect: Len },
    /// Unexpected byte.
    Mismatch {
        /// Type name.
        name: &'static str,
        /// Type byte.
        found: u8,
    },
    /// Too large integer.
    CastOverflow { name: &'static str },
    /// Overflowing 128-bit integers.
    Overflow {
        /// Type of integer.
        name: &'static str,
    },
    /// Decoding bytes/strings might require a borrow.
    RequireBorrowed {
        /// Type name (e.g. "bytes", "str").
        name: &'static str,
    },
    /// Length wasn't large enough. This error comes after attempting to consume the entirety of a
    /// item with a known length and failing to do so.
    RequireLength {
        /// Type name.
        name: &'static str,
        /// Given length.
        found: Len,
    },
    /// Invalid UTF-8.
    InvalidUtf8(core::str::Utf8Error),
    /// Unsupported byte.
    Unsupported {
        name: &'static str,
        /// Unsupported bytes.
        found: u8,
    },
    /// Recursion limit reached.
    DepthOverflow { name: &'static str },
    /// Trailing data.
    TrailingData,
    /// Indefinite sized item was encountered.
    IndefiniteSize,
}

impl<E> From<E> for DecodeError<E> {
    fn from(err: E) -> DecodeError<E> {
        DecodeError::Read(err)
    }
}

impl<E: std::error::Error + 'static> de::Error for DecodeError<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DecodeError::Msg(msg.to_string())
    }
}

impl<E: core::error::Error + 'static> core::error::Error for DecodeError<E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            DecodeError::Msg(_) => None,
            DecodeError::Read(err) => Some(err),
            _ => None,
        }
    }
}

impl<E: fmt::Debug> fmt::Display for DecodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<E: fmt::Debug> From<cbor4ii::core::error::DecodeError<E>> for DecodeError<E> {
    fn from(err: cbor4ii::core::error::DecodeError<E>) -> DecodeError<E> {
        use cbor4ii::core::error::DecodeError as IDecodeError;
        match err {
            IDecodeError::Read(read) => DecodeError::Read(read),
            IDecodeError::Eof { name, expect } => DecodeError::Eof { name, expect },
            IDecodeError::Mismatch { name, found } => DecodeError::Mismatch { name, found },
            IDecodeError::CastOverflow { name } => DecodeError::CastOverflow { name },
            IDecodeError::RequireBorrowed { name } => DecodeError::RequireBorrowed { name },
            IDecodeError::RequireLength { name, found } => {
                DecodeError::RequireLength { name, found }
            }
            IDecodeError::Unsupported { name, found } => DecodeError::Unsupported { name, found },
            IDecodeError::DepthOverflow { name } => DecodeError::DepthOverflow { name },
            // Needed as `cbor4ii::EncodeError` is markes as non_exhaustive
            _ => DecodeError::Msg(err.to_string()),
        }
    }
}

/// Encode and Decode error combined.
#[derive(Debug)]
pub enum CodecError {
    /// A decoding error.
    Decode(DecodeError<Infallible>),
    /// An encoding error.
    Encode(EncodeError<TryReserveError>),
    /// A decoding error.
    DecodeIo(DecodeError<std::io::Error>),
    /// An encoding error.
    EncodeIo(EncodeError<std::io::Error>),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "decode error: {error}"),
            Self::Encode(error) => write!(f, "encode error: {error}"),
            Self::DecodeIo(error) => write!(f, "decode io error: {error}"),
            Self::EncodeIo(error) => write!(f, "encode io error: {error}"),
        }
    }
}

impl std::error::Error for CodecError {}

impl From<DecodeError<Infallible>> for CodecError {
    fn from(error: DecodeError<Infallible>) -> Self {
        Self::Decode(error)
    }
}

impl From<DecodeError<std::io::Error>> for CodecError {
    fn from(error: DecodeError<std::io::Error>) -> Self {
        Self::DecodeIo(error)
    }
}

impl From<EncodeError<TryReserveError>> for CodecError {
    fn from(error: EncodeError<TryReserveError>) -> Self {
        Self::Encode(error)
    }
}

impl From<EncodeError<std::io::Error>> for CodecError {
    fn from(error: EncodeError<std::io::Error>) -> Self {
        Self::EncodeIo(error)
    }
}

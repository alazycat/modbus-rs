/// Errors that can occur when encoding a PDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// The provided buffer is too small to hold the encoded PDU.
    BufferTooSmall,
}

/// Errors that can occur when decoding a PDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The buffer does not contain enough bytes for the expected PDU.
    InvalidLength,
    /// The quantity field is outside the allowed range for the function code.
    InvalidQuantity,
    /// A value field is outside the allowed range for the function code.
    InvalidValue,
    /// The function code in the response does not match the request.
    UnknownFunctionCode,
}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

#[cfg(feature = "std")]
impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

#[cfg(feature = "std")]
impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InvalidLength => write!(f, "invalid length"),
            DecodeError::InvalidQuantity => write!(f, "invalid quantity"),
            DecodeError::InvalidValue => write!(f, "invalid value"),
            DecodeError::UnknownFunctionCode => write!(f, "unknown function code"),
        }
    }
}

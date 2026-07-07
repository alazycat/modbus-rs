#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod error;
pub mod function;
pub mod function_codes;

pub use error::{DecodeError, EncodeError};
pub use function_codes::read_discrete_inputs::{
    ReadDiscreteInputsRequest, ReadDiscreteInputsResponse,
};
pub use function_codes::read_coils::{ReadCoilsRequest, ReadCoilsResponse};

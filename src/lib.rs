#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod error;
pub mod function;
pub mod function_codes;

pub use error::{DecodeError, EncodeError};
pub use function_codes::read_fifo_queue::{ReadFifoQueueRequest, ReadFifoQueueResponse};
pub use function_codes::read_write_multiple_registers::{
    ReadWriteMultipleRegistersRequest, ReadWriteMultipleRegistersResponse,
};
pub use function_codes::mask_write_register::{
    MaskWriteRegisterRequest, MaskWriteRegisterResponse,
};
pub use function_codes::write_multiple_registers::{
    WriteMultipleRegistersRequest, WriteMultipleRegistersResponse,
};
pub use function_codes::write_single_register::{
    WriteSingleRegisterRequest, WriteSingleRegisterResponse,
};
pub use function_codes::read_input_registers::{
    ReadInputRegistersRequest, ReadInputRegistersResponse,
};
pub use function_codes::read_holding_registers::{
    ReadHoldingRegistersRequest, ReadHoldingRegistersResponse,
};
pub use function_codes::write_multiple_coils::{
    WriteMultipleCoilsRequest, WriteMultipleCoilsResponse,
};
pub use function_codes::write_single_coil::{
    WriteSingleCoilRequest, WriteSingleCoilResponse,
};
pub use function_codes::read_discrete_inputs::{
    ReadDiscreteInputsRequest, ReadDiscreteInputsResponse,
};
pub use function_codes::read_coils::{ReadCoilsRequest, ReadCoilsResponse};

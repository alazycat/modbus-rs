#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod error;
pub mod exception;
pub mod function;
pub mod function_codes;

#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "ascii")]
pub mod ascii;

pub use error::{DecodeError, EncodeError};
pub use exception::{ExceptionCode, ExceptionResponse};
pub use function_codes::encapsulated_interface_transport::{
    EncapsulatedInterfaceTransportRequest, EncapsulatedInterfaceTransportResponse,
    MEI_TYPE_CANOPEN_GENERAL_REFERENCE, MEI_TYPE_READ_DEVICE_IDENTIFICATION,
    READ_DEVICE_ID_CODE_BASIC, READ_DEVICE_ID_CODE_EXTENDED,
    READ_DEVICE_ID_CODE_REGULAR, READ_DEVICE_ID_CODE_SPECIFIC,
};
pub use function_codes::write_file_record::{
    WriteFileRecordRequest, WriteFileRecordResponse, WriteFileRecordSubRequest,
    WriteFileRecordSubResponse,
};
pub use function_codes::read_file_record::{
    ReadFileRecordRequest, ReadFileRecordResponse, ReadFileRecordSubRequest,
    ReadFileRecordSubResponse,
};
pub use function_codes::report_server_id::{
    ReportServerIdRequest, ReportServerIdResponse,
};
pub use function_codes::get_comm_event_log::{
    GetCommEventLogRequest, GetCommEventLogResponse,
};
pub use function_codes::get_comm_event_counter::{
    GetCommEventCounterRequest, GetCommEventCounterResponse,
};
pub use function_codes::diagnostics::{
    DiagnosticsRequest, DiagnosticsResponse,
};
pub use function_codes::read_exception_status::{
    ReadExceptionStatusRequest, ReadExceptionStatusResponse,
};
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

//! Asynchronous Modbus client core.

#![cfg(feature = "async")]

use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use super::{AsyncAduAdapter, AsyncRtuAduAdapter, ClientConfig, ClientError};
#[cfg(test)]
use crate::exception::ExceptionResponse;
use crate::function_codes::diagnostics::{DiagnosticsRequest, DiagnosticsResponse};
use crate::function_codes::encapsulated_interface_transport::{
    EncapsulatedInterfaceTransportRequest, EncapsulatedInterfaceTransportResponse,
};
use crate::function_codes::get_comm_event_counter::{
    GetCommEventCounterRequest, GetCommEventCounterResponse,
};
use crate::function_codes::get_comm_event_log::{GetCommEventLogRequest, GetCommEventLogResponse};
use crate::function_codes::mask_write_register::{
    MaskWriteRegisterRequest, MaskWriteRegisterResponse,
};
use crate::function_codes::read_coils::{ReadCoilsRequest, ReadCoilsResponse};
use crate::function_codes::read_discrete_inputs::{
    ReadDiscreteInputsRequest, ReadDiscreteInputsResponse,
};
use crate::function_codes::read_exception_status::{
    ReadExceptionStatusRequest, ReadExceptionStatusResponse,
};
use crate::function_codes::read_fifo_queue::{ReadFifoQueueRequest, ReadFifoQueueResponse};
use crate::function_codes::read_file_record::{
    ReadFileRecordRequest, ReadFileRecordResponse, ReadFileRecordSubRequest,
    ReadFileRecordSubResponse,
};
use crate::function_codes::read_holding_registers::{
    ReadHoldingRegistersRequest, ReadHoldingRegistersResponse,
};
use crate::function_codes::read_input_registers::{
    ReadInputRegistersRequest, ReadInputRegistersResponse,
};
use crate::function_codes::read_write_multiple_registers::{
    ReadWriteMultipleRegistersRequest, ReadWriteMultipleRegistersResponse,
};
use crate::function_codes::report_server_id::{ReportServerIdRequest, ReportServerIdResponse};
use crate::function_codes::write_file_record::{
    WriteFileRecordRequest, WriteFileRecordResponse, WriteFileRecordSubRequest,
    WriteFileRecordSubResponse,
};
use crate::function_codes::write_multiple_coils::{
    WriteMultipleCoilsRequest, WriteMultipleCoilsResponse,
};
use crate::function_codes::write_multiple_registers::{
    WriteMultipleRegistersRequest, WriteMultipleRegistersResponse,
};
use crate::function_codes::write_single_coil::{WriteSingleCoilRequest, WriteSingleCoilResponse};
use crate::function_codes::write_single_register::{
    WriteSingleRegisterRequest, WriteSingleRegisterResponse,
};
use crate::macros::impl_client_methods;
use crate::transport::AsyncTransport;
#[cfg(feature = "helpers")]
use crate::{helpers, helpers::Endian};

/// Generic asynchronous Modbus client.
///
/// The client dispatches request PDUs through an [`AsyncAduAdapter`], waits for
/// the response, and performs basic response validation.
#[derive(Debug)]
pub struct AsyncClientCore<A: AsyncAduAdapter> {
    adapter: A,
    #[cfg(feature = "helpers")]
    config: ClientConfig,
}

impl<A: AsyncAduAdapter> AsyncClientCore<A> {
    /// Create a client around an adapter with the default configuration.
    pub fn new(adapter: A) -> Self {
        Self::with_config(adapter, ClientConfig::default())
    }

    /// Create a client around an adapter with a custom configuration.
    pub fn with_config(adapter: A, config: ClientConfig) -> Self {
        #[cfg(not(feature = "helpers"))]
        let _ = config;
        Self {
            adapter,
            #[cfg(feature = "helpers")]
            config,
        }
    }

    /// Dispatch a request PDU to `unit_id` and return the response PDU.
    ///
    /// The request PDU must begin with the function code. The returned response
    /// PDU also begins with the function code, unless the server replied with
    /// an exception.
    pub async fn dispatch(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<Vec<u8>, ClientError> {
        if request_pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        let request_function = request_pdu[0];
        let response_pdu = self.adapter.send_receive(unit_id, request_pdu).await?;
        super::validate_response_function(request_function, &response_pdu)?;
        Ok(response_pdu)
    }

    impl_client_methods!([async] [.await]);
}

#[cfg(feature = "helpers")]
impl<A: AsyncAduAdapter> AsyncClientCore<A> {
    /// Read a single holding register as a `u16`.
    pub async fn read_holding_registers_u16(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<u16, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, 1).await?;
        helpers::u16_from_bytes(&bytes, self.config.endian).map_err(ClientError::from)
    }

    /// Read a single holding register as an `i16`.
    pub async fn read_holding_registers_i16(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<i16, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, 1).await?;
        helpers::i16_from_bytes(&bytes, self.config.endian).map_err(ClientError::from)
    }

    /// Read two holding registers as a `u32` using the configured endianness and word order.
    pub async fn read_holding_registers_u32(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<u32, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, 2).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::u32_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read two holding registers as an `i32` using the configured endianness and word order.
    pub async fn read_holding_registers_i32(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<i32, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, 2).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::i32_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read two holding registers as an `f32` using the configured endianness and word order.
    pub async fn read_holding_registers_f32(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<f32, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, 2).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::f32_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read four holding registers as a `u64` using the configured endianness and word order.
    pub async fn read_holding_registers_u64(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<u64, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, 4).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::u64_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read four holding registers as an `i64` using the configured endianness and word order.
    pub async fn read_holding_registers_i64(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<i64, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, 4).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::i64_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read four holding registers as an `f64` using the configured endianness and word order.
    pub async fn read_holding_registers_f64(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<f64, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, 4).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::f64_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read `quantity` holding registers as a NUL-terminated string.
    pub async fn read_holding_registers_string(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<String, ClientError> {
        let bytes = self.read_holding_registers(unit_id, address, quantity).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::string_from_registers(&words, self.config.endian).map_err(ClientError::from)
    }

    /// Read a single input register as a `u16`.
    pub async fn read_input_registers_u16(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<u16, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, 1).await?;
        helpers::u16_from_bytes(&bytes, self.config.endian).map_err(ClientError::from)
    }

    /// Read a single input register as an `i16`.
    pub async fn read_input_registers_i16(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<i16, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, 1).await?;
        helpers::i16_from_bytes(&bytes, self.config.endian).map_err(ClientError::from)
    }

    /// Read two input registers as a `u32` using the configured endianness and word order.
    pub async fn read_input_registers_u32(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<u32, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, 2).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::u32_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read two input registers as an `i32` using the configured endianness and word order.
    pub async fn read_input_registers_i32(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<i32, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, 2).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::i32_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read two input registers as an `f32` using the configured endianness and word order.
    pub async fn read_input_registers_f32(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<f32, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, 2).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::f32_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read four input registers as a `u64` using the configured endianness and word order.
    pub async fn read_input_registers_u64(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<u64, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, 4).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::u64_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read four input registers as an `i64` using the configured endianness and word order.
    pub async fn read_input_registers_i64(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<i64, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, 4).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::i64_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read four input registers as an `f64` using the configured endianness and word order.
    pub async fn read_input_registers_f64(
        &mut self,
        unit_id: u8,
        address: u16,
    ) -> Result<f64, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, 4).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::f64_from_registers(&words, self.config.endian, self.config.word_order)
            .map_err(ClientError::from)
    }

    /// Read `quantity` input registers as a NUL-terminated string.
    pub async fn read_input_registers_string(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<String, ClientError> {
        let bytes = self.read_input_registers(unit_id, address, quantity).await?;
        let words = bytes_to_words(&bytes, self.config.endian)?;
        helpers::string_from_registers(&words, self.config.endian).map_err(ClientError::from)
    }
}

#[cfg(feature = "helpers")]
fn bytes_to_words(bytes: &[u8], endian: Endian) -> Result<Vec<u16>, ClientError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(ClientError::Decode(crate::error::DecodeError::InvalidLength));
    }
    bytes
        .chunks_exact(2)
        .map(|chunk| helpers::u16_from_bytes(chunk, endian))
        .collect::<Result<Vec<_>, _>>()
        .map_err(ClientError::from)
}

#[cfg(feature = "helpers")]
impl<A: AsyncAduAdapter> AsyncClientCore<A> {
    /// Write a `u16` value to a single holding register.
    pub async fn write_multiple_registers_u16(
        &mut self,
        unit_id: u8,
        address: u16,
        value: u16,
    ) -> Result<(), ClientError> {
        self.write_registers(unit_id, address, &[value]).await
    }

    /// Write an `i16` value to a single holding register.
    pub async fn write_multiple_registers_i16(
        &mut self,
        unit_id: u8,
        address: u16,
        value: i16,
    ) -> Result<(), ClientError> {
        self.write_registers(unit_id, address, &[value as u16]).await
    }

    /// Write a `u32` value to two holding registers using the configured endianness and word order.
    pub async fn write_multiple_registers_u32(
        &mut self,
        unit_id: u8,
        address: u16,
        value: u32,
    ) -> Result<(), ClientError> {
        let regs = helpers::u32_to_registers(value, self.config.endian, self.config.word_order);
        self.write_registers(unit_id, address, &regs).await
    }

    /// Write an `i32` value to two holding registers using the configured endianness and word order.
    pub async fn write_multiple_registers_i32(
        &mut self,
        unit_id: u8,
        address: u16,
        value: i32,
    ) -> Result<(), ClientError> {
        let regs = helpers::i32_to_registers(value, self.config.endian, self.config.word_order);
        self.write_registers(unit_id, address, &regs).await
    }

    /// Write an `f32` value to two holding registers using the configured endianness and word order.
    pub async fn write_multiple_registers_f32(
        &mut self,
        unit_id: u8,
        address: u16,
        value: f32,
    ) -> Result<(), ClientError> {
        let regs = helpers::f32_to_registers(value, self.config.endian, self.config.word_order);
        self.write_registers(unit_id, address, &regs).await
    }

    /// Write a `u64` value to four holding registers using the configured endianness and word order.
    pub async fn write_multiple_registers_u64(
        &mut self,
        unit_id: u8,
        address: u16,
        value: u64,
    ) -> Result<(), ClientError> {
        let regs = helpers::u64_to_registers(value, self.config.endian, self.config.word_order);
        self.write_registers(unit_id, address, &regs).await
    }

    /// Write an `i64` value to four holding registers using the configured endianness and word order.
    pub async fn write_multiple_registers_i64(
        &mut self,
        unit_id: u8,
        address: u16,
        value: i64,
    ) -> Result<(), ClientError> {
        let regs = helpers::i64_to_registers(value, self.config.endian, self.config.word_order);
        self.write_registers(unit_id, address, &regs).await
    }

    /// Write an `f64` value to four holding registers using the configured endianness and word order.
    pub async fn write_multiple_registers_f64(
        &mut self,
        unit_id: u8,
        address: u16,
        value: f64,
    ) -> Result<(), ClientError> {
        let regs = helpers::f64_to_registers(value, self.config.endian, self.config.word_order);
        self.write_registers(unit_id, address, &regs).await
    }

    /// Write a string to holding registers, padded to `pad_to` registers if non-zero.
    pub async fn write_multiple_registers_string(
        &mut self,
        unit_id: u8,
        address: u16,
        value: &str,
        pad_to: usize,
    ) -> Result<(), ClientError> {
        let regs = helpers::string_to_registers(value, self.config.endian, pad_to)
            .map_err(ClientError::from)?;
        self.write_registers(unit_id, address, &regs).await
    }
}

/// Asynchronous RTU Modbus client.
///
/// This is a backward-compatible newtype around [`AsyncClientCore`] paired with
/// an asynchronous RTU ADU adapter.
#[derive(Debug)]
pub struct AsyncClient<T: AsyncTransport>(AsyncClientCore<AsyncRtuAduAdapter<T>>);

impl<T: AsyncTransport> AsyncClient<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: ClientConfig) -> Self {
        Self(AsyncClientCore::with_config(
            AsyncRtuAduAdapter::with_config(transport, config),
            config,
        ))
    }
}

impl<T: AsyncTransport> Deref for AsyncClient<T> {
    type Target = AsyncClientCore<AsyncRtuAduAdapter<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: AsyncTransport> DerefMut for AsyncClient<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(all(feature = "rtu", feature = "tcp"))]
impl AsyncClient<crate::rtu_transport::AsyncRtuTransport<tokio::net::TcpStream>> {
    /// Connect to a remote RTU-over-TCP server asynchronously.
    ///
    /// This opens a plain TCP connection and wraps it with RTU framing. The
    /// resulting client is functionally identical to an RTU serial client, but
    /// the bytes travel over TCP.
    pub async fn connect_rtu_over_tcp(
        addr: impl tokio::net::ToSocketAddrs,
        config: ClientConfig,
    ) -> Result<Self, ClientError> {
        let stream = tokio::net::TcpStream::connect(addr)
            .await
            .map_err(crate::transport::TransportError::Io)?;
        let transport = crate::rtu_transport::AsyncRtuTransport::new(stream);
        Ok(Self::with_config(transport, config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function_codes::read_coils::{ReadCoilsRequest, ReadCoilsResponse};
    use crate::rtu::RtuAdu;
    use crate::transport::TransportError;
    use alloc::collections::VecDeque;
    use core::time::Duration;

    struct MockTransport {
        sent: Vec<Vec<u8>>,
        responses: VecDeque<Vec<u8>>,
    }

    impl MockTransport {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                sent: Vec::new(),
                responses: responses.into(),
            }
        }
    }

    impl AsyncTransport for MockTransport {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            self.sent.push(data.to_vec());
            Ok(())
        }

        async fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let resp = self
                .responses
                .pop_front()
                .ok_or(TransportError::Disconnected)?;
            let n = resp.len().min(buf.len());
            buf[..n].copy_from_slice(&resp[..n]);
            Ok(n)
        }
    }

    #[tokio::test]
    async fn dispatch_read_coils_roundtrip() {
        let request_pdu = {
            let req = ReadCoilsRequest::new(0x0000, 10).unwrap();
            let mut buf = [0u8; 5];
            let n = req.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let response_pdu = {
            let resp = ReadCoilsResponse {
                coil_status: vec![0b11001011, 0b00000010],
            };
            let mut buf = [0u8; 4];
            let n = resp.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let response_adu = {
            let adu = RtuAdu::new(0x01, response_pdu.clone());
            let mut buf = [0u8; 512];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsyncClient::new(MockTransport::new(vec![response_adu]));
        let pdu = client.dispatch(0x01, &request_pdu).await.unwrap();
        assert_eq!(pdu, response_pdu);

        let decoded = ReadCoilsResponse::decode(&pdu).unwrap();
        assert_eq!(decoded.coil_status, vec![0b11001011, 0b00000010]);
    }

    #[tokio::test]
    async fn dispatch_returns_exception() {
        let request_pdu = {
            let req = ReadCoilsRequest::new(0x0000, 10).unwrap();
            let mut buf = [0u8; 5];
            let n = req.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let exception_pdu = {
            let exc =
                ExceptionResponse::new(0x01, crate::exception::ExceptionCode::IllegalDataAddress);
            let mut buf = [0u8; 2];
            let n = exc.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let response_adu = {
            let adu = RtuAdu::new(0x01, exception_pdu);
            let mut buf = [0u8; 512];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsyncClient::new(MockTransport::new(vec![response_adu]));
        let err = client.dispatch(0x01, &request_pdu).await.unwrap_err();
        assert!(matches!(err, ClientError::Exception(_)));
    }

    #[tokio::test]
    async fn dispatch_propagates_timeout() {
        struct TimeoutTransport;
        impl AsyncTransport for TimeoutTransport {
            async fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }
            async fn recv(
                &mut self,
                _buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                Err(TransportError::Timeout)
            }
        }

        let mut client = AsyncClient::new(TimeoutTransport);
        let err = client
            .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x0A])
            .await
            .unwrap_err();
        assert!(matches!(err, ClientError::Timeout));
    }

    #[tokio::test]
    async fn dispatch_rejects_wrong_slave() {
        let response_adu = {
            let pdu = {
                let resp = ReadCoilsResponse {
                    coil_status: vec![0x01],
                };
                let mut buf = [0u8; 3];
                let n = resp.encode(&mut buf).unwrap();
                buf[..n].to_vec()
            };
            let adu = RtuAdu::new(0x02, pdu);
            let mut buf = [0u8; 512];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsyncClient::new(MockTransport::new(vec![response_adu]));
        let err = client
            .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x01])
            .await
            .unwrap_err();
        assert!(matches!(err, ClientError::InvalidResponse));
    }
}

#[cfg(all(test, feature = "helpers"))]
mod typed_helpers_tests {
    use super::*;
    use crate::helpers::{Endian, WordOrder};
    use crate::rtu::RtuAdu;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::transport::{AsyncTransport, TransportError};
    use core::time::Duration;

    struct AsyncLoopbackTransport {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsyncLoopbackTransport {
        fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl AsyncTransport for AsyncLoopbackTransport {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = RtuAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = RtuAdu::new(request.address, pdu_response[..n].to_vec());
            let mut adu = [0u8; 512];
            let m = response
                .encode(&mut adu)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(adu[..m].to_vec());
            Ok(())
        }

        async fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let data = self.pending.take().ok_or(TransportError::Disconnected)?;
            if buf.len() < data.len() {
                return Err(TransportError::Disconnected);
            }
            buf[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
    }

    fn holding_client(
        regs: &[u16],
        config: ClientConfig,
    ) -> AsyncClient<AsyncLoopbackTransport> {
        let mut store = MemoryStore::new(0, 0, 8, 8);
        store.write_registers(0, regs).unwrap();
        AsyncClient::with_config(AsyncLoopbackTransport::new(Server::new(store)), config)
    }

    fn input_client(
        regs: &[u16],
        config: ClientConfig,
    ) -> AsyncClient<AsyncLoopbackTransport> {
        let mut store = MemoryStore::new(0, 0, 0, 8);
        store.write_input_registers(0, regs).unwrap();
        AsyncClient::with_config(AsyncLoopbackTransport::new(Server::new(store)), config)
    }

    #[tokio::test]
    async fn read_holding_registers_u32_big_endian_msf() {
        let mut client = holding_client(&[0x1234, 0x5678], ClientConfig::default());
        assert_eq!(
            client.read_holding_registers_u32(0x01, 0).await.unwrap(),
            0x12345678
        );
    }

    #[tokio::test]
    async fn read_holding_registers_u32_little_endian_lsf() {
        let mut config = ClientConfig::default();
        config.endian = Endian::Little;
        config.word_order = WordOrder::LeastSignificantFirst;
        let mut client = holding_client(&[0x5678, 0x1234], config);
        assert_eq!(
            client.read_holding_registers_u32(0x01, 0).await.unwrap(),
            0x12345678
        );
    }

    #[tokio::test]
    async fn read_holding_registers_f32_roundtrip() {
        let value = 2.7182817f32;
        let regs = crate::helpers::f32_to_registers(value, Endian::Big, WordOrder::MostSignificantFirst);
        let mut client = holding_client(&regs, ClientConfig::default());
        assert_eq!(
            client.read_holding_registers_f32(0x01, 0).await.unwrap(),
            value
        );
    }

    #[tokio::test]
    async fn read_holding_registers_string_stops_at_nul() {
        let regs = crate::helpers::string_to_registers("Hi", Endian::Big, 4).unwrap();
        let mut client = holding_client(&regs, ClientConfig::default());
        assert_eq!(
            client.read_holding_registers_string(0x01, 0, 4).await.unwrap(),
            "Hi"
        );
    }

    #[tokio::test]
    async fn read_input_registers_u32_uses_config() {
        let mut client = input_client(&[0x1234, 0x5678], ClientConfig::default());
        assert_eq!(
            client.read_input_registers_u32(0x01, 0).await.unwrap(),
            0x12345678
        );
    }

    #[tokio::test]
    async fn write_and_read_holding_registers_u32_roundtrip() {
        let value = 0xCAFEBABEu32;
        let mut client = holding_client(&[0, 0], ClientConfig::default());
        client
            .write_multiple_registers_u32(0x01, 0, value)
            .await
            .unwrap();
        assert_eq!(
            client.read_holding_registers_u32(0x01, 0).await.unwrap(),
            value
        );
    }

    #[tokio::test]
    async fn write_and_read_holding_registers_f32_roundtrip() {
        let value = -1.5f32;
        let mut client = holding_client(&[0, 0], ClientConfig::default());
        client
            .write_multiple_registers_f32(0x01, 0, value)
            .await
            .unwrap();
        assert_eq!(
            client.read_holding_registers_f32(0x01, 0).await.unwrap(),
            value
        );
    }

    #[tokio::test]
    async fn write_and_read_holding_registers_string_roundtrip() {
        let mut client = holding_client(&[0, 0, 0, 0], ClientConfig::default());
        client
            .write_multiple_registers_string(0x01, 0, "Hello", 4)
            .await
            .unwrap();
        assert_eq!(
            client.read_holding_registers_string(0x01, 0, 4).await.unwrap(),
            "Hello"
        );
    }
}

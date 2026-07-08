//! Macros used internally by the Modbus client implementations.

/// Generate the high-level Modbus client methods for a synchronous or
/// asynchronous [`ClientCore`](crate::client::ClientCore)/
/// [`AsyncClientCore`](crate::client::AsyncClientCore).
///
/// Call with an empty async/await pair for the synchronous implementation:
///
/// ```ignore
/// impl_client_methods!([] []);
/// ```
///
/// Call with `async`/`.await` for the asynchronous implementation:
///
/// ```ignore
/// impl_client_methods!([async] [.await]);
/// ```
#[macro_export]
macro_rules! impl_client_methods {
    ([$($async:tt)*] [$($await:tt)*]) => {
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            read_coils,
            (unit_id: u8, address: u16, quantity: u16) -> Vec<u8>,
            {
                let req = ReadCoilsRequest::new(address, quantity).map_err(ClientError::Decode)?;
                (req, [0u8; 5])
            },
            ReadCoilsResponse,
            [coil_status]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            read_discrete_inputs,
            (unit_id: u8, address: u16, quantity: u16) -> Vec<u8>,
            {
                let req = ReadDiscreteInputsRequest::new(address, quantity).map_err(ClientError::Decode)?;
                (req, [0u8; 5])
            },
            ReadDiscreteInputsResponse,
            [input_status]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            read_holding_registers,
            (unit_id: u8, address: u16, quantity: u16) -> Vec<u8>,
            {
                let req = ReadHoldingRegistersRequest::new(address, quantity).map_err(ClientError::Decode)?;
                (req, [0u8; 5])
            },
            ReadHoldingRegistersResponse,
            [register_values]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            read_input_registers,
            (unit_id: u8, address: u16, quantity: u16) -> Vec<u8>,
            {
                let req = ReadInputRegistersRequest::new(address, quantity).map_err(ClientError::Decode)?;
                (req, [0u8; 5])
            },
            ReadInputRegistersResponse,
            [register_values]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            write_coil,
            (unit_id: u8, address: u16, value: bool) -> (),
            {
                let raw = if value { WriteSingleCoilRequest::ON } else { WriteSingleCoilRequest::OFF };
                let req = WriteSingleCoilRequest::new(address, raw).map_err(ClientError::Decode)?;
                (req, [0u8; 5])
            },
            WriteSingleCoilResponse,
            []
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            write_register,
            (unit_id: u8, address: u16, value: u16) -> (),
            {
                let req = WriteSingleRegisterRequest::new(address, value);
                (req, [0u8; 5])
            },
            WriteSingleRegisterResponse,
            []
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            write_coils,
            (unit_id: u8, address: u16, values: &[bool]) -> (),
            {
                let outputs = $crate::client::pack_bits(values);
                let quantity = values.len() as u16;
                let req = WriteMultipleCoilsRequest::new(address, quantity, outputs)
                    .map_err(ClientError::Decode)?;
                let buf = vec![0u8; 6 + req.outputs.len()];
                (req, buf)
            },
            WriteMultipleCoilsResponse,
            []
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            write_registers,
            (unit_id: u8, address: u16, values: &[u16]) -> (),
            {
                let mut register_values = Vec::with_capacity(values.len() * 2);
                for &value in values {
                    register_values.extend_from_slice(&value.to_be_bytes());
                }
                let quantity = values.len() as u16;
                let req = WriteMultipleRegistersRequest::new(address, quantity, register_values)
                    .map_err(ClientError::Decode)?;
                let buf = vec![0u8; 6 + req.register_values.len()];
                (req, buf)
            },
            WriteMultipleRegistersResponse,
            []
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            read_exception_status,
            (unit_id: u8) -> u8,
            {
                let req = ReadExceptionStatusRequest;
                (req, [0u8; 1])
            },
            ReadExceptionStatusResponse,
            [data]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            diagnostics,
            (unit_id: u8, sub_function: u16, data: u16) -> (u16, u16),
            {
                let req = DiagnosticsRequest::new(sub_function, data);
                (req, [0u8; 5])
            },
            DiagnosticsResponse,
            [sub_function, data]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            get_comm_event_counter,
            (unit_id: u8) -> (u16, u16),
            {
                let req = GetCommEventCounterRequest;
                (req, [0u8; 1])
            },
            GetCommEventCounterResponse,
            [status, event_count]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            get_comm_event_log,
            (unit_id: u8) -> (u16, u16, u16, Vec<u8>),
            {
                let req = GetCommEventLogRequest;
                (req, [0u8; 1])
            },
            GetCommEventLogResponse,
            [status, event_count, message_count, events]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            report_server_id,
            (unit_id: u8) -> Vec<u8>,
            {
                let req = ReportServerIdRequest;
                (req, [0u8; 1])
            },
            ReportServerIdResponse,
            [data]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            mask_write_register,
            (unit_id: u8, reference_address: u16, and_mask: u16, or_mask: u16) -> (u16, u16, u16),
            {
                let req = MaskWriteRegisterRequest::new(reference_address, and_mask, or_mask);
                (req, [0u8; 7])
            },
            MaskWriteRegisterResponse,
            [reference_address, and_mask, or_mask]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            read_write_multiple_registers,
            (unit_id: u8, read_address: u16, read_quantity: u16, write_address: u16, write_values: &[u16]) -> Vec<u8>,
            {
                let mut write_register_values = Vec::with_capacity(write_values.len() * 2);
                for &value in write_values {
                    write_register_values.extend_from_slice(&value.to_be_bytes());
                }
                let write_quantity = write_values.len() as u16;
                let req = ReadWriteMultipleRegistersRequest::new(
                    read_address,
                    read_quantity,
                    write_address,
                    write_quantity,
                    write_register_values,
                )
                .map_err(ClientError::Decode)?;
                let buf = vec![0u8; 10 + req.write_values.len()];
                (req, buf)
            },
            ReadWriteMultipleRegistersResponse,
            [register_values]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            read_fifo_queue,
            (unit_id: u8, fifo_pointer_address: u16) -> (u16, Vec<u8>),
            {
                let req = ReadFifoQueueRequest::new(fifo_pointer_address);
                (req, [0u8; 3])
            },
            ReadFifoQueueResponse,
            [fifo_count, register_values]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            read_file_record,
            (unit_id: u8, sub_requests: &[ReadFileRecordSubRequest]) -> Vec<ReadFileRecordSubResponse>,
            {
                let req = ReadFileRecordRequest::new(sub_requests.to_vec());
                let buf = vec![0u8; 2 + sub_requests.len() * 7];
                (req, buf)
            },
            ReadFileRecordResponse,
            [sub_responses]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            write_file_record,
            (unit_id: u8, sub_requests: &[WriteFileRecordSubRequest]) -> Vec<WriteFileRecordSubResponse>,
            {
                let req = WriteFileRecordRequest::new(sub_requests.to_vec());
                let buf = vec![0u8; 2 + sub_requests.iter().map(|s| 7 + s.record_data.len()).sum::<usize>()];
                (req, buf)
            },
            WriteFileRecordResponse,
            [sub_responses]
        }
        impl_client_methods! {
            @method [$($async)*] [$($await)*],
            encapsulated_interface_transport,
            (unit_id: u8, mei_type: u8, data: &[u8]) -> (u8, Vec<u8>),
            {
                let req = EncapsulatedInterfaceTransportRequest::new(mei_type, data.to_vec());
                let buf = vec![0u8; 2 + data.len()];
                (req, buf)
            },
            EncapsulatedInterfaceTransportResponse,
            [mei_type, data]
        }
    };

    (@method [$($async:tt)*] [$($await:tt)*], $name:ident, ($unit_id:ident: u8 $(, $arg_name:ident: $arg_ty:ty)*) -> $ret:ty, $req:block, $resp:ty, [$($field:ident),*]) => {
        pub $($async)* fn $name(
            &mut self,
            $unit_id: u8,
            $($arg_name: $arg_ty),*
        ) -> Result<$ret, ClientError> {
            let (req, mut buf) = $req;
            let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
            let pdu = self.dispatch($unit_id, &buf[..n]) $($await)* ?;
            #[allow(unused_variables)]
            let resp = <$resp>::decode(&pdu).map_err(ClientError::Decode)?;
            Ok(($(resp.$field),*))
        }
    };
}

/// Re-export the macro at the crate root so `crate::impl_client_methods!` works.
pub use impl_client_methods;

/// Generate the sync/async ADU adapter boilerplate for a Modbus protocol.
///
/// The macro emits the adapter struct, constructors, and the `AduAdapter`/
/// `AsyncAduAdapter` implementation. It supports two protocol shapes:
/// - `transaction` for TCP/UDP (MBAP header with transaction ID)
/// - `no_transaction` for RTU/ASCII (address field in ADU)
///
/// Example for RTU (sync):
///
/// ```ignore
/// impl_adu_adapter! {
///     [] [],
///     /// Synchronous RTU ADU adapter.
///     RtuAduAdapter,
///     crate::rtu::RtuAdu,
///     no_transaction
/// }
/// ```
///
/// Example for TCP (async):
///
/// ```ignore
/// impl_adu_adapter! {
///     [async] [.await],
///     /// Asynchronous TCP ADU adapter.
///     AsyncTcpAduAdapter,
///     crate::tcp::TcpAdu,
///     transaction
/// }
/// ```
#[macro_export]
macro_rules! impl_adu_adapter {
    // Public arms that select the sync or async transport/ADU traits.
    ([] [], $(#[$meta:meta])* $adapter:ident, $adu:ty, no_transaction) => {
        impl_adu_adapter! {
            @internal [] [],
            $(#[$meta])* $adapter, $adu,
            $crate::transport::Transport,
            $crate::client::AduAdapter,
            no_transaction
        }
    };
    ([async] [.await], $(#[$meta:meta])* $adapter:ident, $adu:ty, no_transaction) => {
        impl_adu_adapter! {
            @internal [async] [.await],
            $(#[$meta])* $adapter, $adu,
            $crate::transport::AsyncTransport,
            $crate::client::AsyncAduAdapter,
            no_transaction
        }
    };
    ([] [], $(#[$meta:meta])* $adapter:ident, $adu:ty, transaction) => {
        impl_adu_adapter! {
            @internal [] [],
            $(#[$meta])* $adapter, $adu,
            $crate::transport::Transport,
            $crate::client::AduAdapter,
            transaction
        }
    };
    ([async] [.await], $(#[$meta:meta])* $adapter:ident, $adu:ty, transaction) => {
        impl_adu_adapter! {
            @internal [async] [.await],
            $(#[$meta])* $adapter, $adu,
            $crate::transport::AsyncTransport,
            $crate::client::AsyncAduAdapter,
            transaction
        }
    };

    // Internal arm: protocol without transaction ID (RTU, ASCII).
    (@internal [$($async:tt)*] [$($await:tt)*], $(#[$meta:meta])* $adapter:ident, $adu:ty, $transport:path, $trait:path, no_transaction) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $adapter<T: $transport> {
            transport: T,
            config: $crate::client::ClientConfig,
        }

        impl<T: $transport> $adapter<T> {
            /// Create an adapter with the default configuration.
            pub fn new(transport: T) -> Self {
                Self::with_config(transport, $crate::client::ClientConfig::default())
            }

            /// Create an adapter with a custom configuration.
            pub fn with_config(transport: T, config: $crate::client::ClientConfig) -> Self {
                Self { transport, config }
            }
        }

        impl<T: $transport> $trait for $adapter<T> {
            $($async)* fn send_receive(
                &mut self,
                unit_id: u8,
                request_pdu: &[u8],
            ) -> Result<Vec<u8>, $crate::client::ClientError> {
                let adu = <$adu>::new(unit_id, request_pdu.to_vec());
                let mut tx = [0u8; 512];
                let n = adu.encode(&mut tx).map_err($crate::client::ClientError::Encode)?;
                self.transport.send(&tx[..n]) $($await)* ?;

                let mut rx = [0u8; 512];
                let m = self.transport.recv(&mut rx, self.config.timeout) $($await)* ?;
                if m == 0 {
                    return Err($crate::client::ClientError::Transport(
                        $crate::transport::TransportError::Disconnected,
                    ));
                }
                let response = <$adu>::decode(&rx[..m]).map_err($crate::client::ClientError::Decode)?;
                if response.address != unit_id {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                if response.pdu.is_empty() {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                Ok(response.pdu)
            }
        }
    };

    // Internal arm: protocol with transaction ID (TCP, UDP).
    (@internal [$($async:tt)*] [$($await:tt)*], $(#[$meta:meta])* $adapter:ident, $adu:ty, $transport:path, $trait:path, transaction) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $adapter<T: $transport> {
            transport: T,
            config: $crate::client::ClientConfig,
            next_transaction_id: u16,
        }

        impl<T: $transport> $adapter<T> {
            /// Create an adapter with the default configuration.
            pub fn new(transport: T) -> Self {
                Self::with_config(transport, $crate::client::ClientConfig::default())
            }

            /// Create an adapter with a custom configuration.
            pub fn with_config(transport: T, config: $crate::client::ClientConfig) -> Self {
                Self {
                    transport,
                    config,
                    next_transaction_id: 1,
                }
            }
        }

        impl<T: $transport> $trait for $adapter<T> {
            $($async)* fn send_receive(
                &mut self,
                unit_id: u8,
                request_pdu: &[u8],
            ) -> Result<Vec<u8>, $crate::client::ClientError> {
                let transaction_id = self.next_transaction_id;
                self.next_transaction_id = self.next_transaction_id.wrapping_add(1);

                let adu = <$adu>::new(transaction_id, unit_id, request_pdu.to_vec());
                let mut tx = [0u8; 512];
                let n = adu.encode(&mut tx).map_err($crate::client::ClientError::Encode)?;
                self.transport.send(&tx[..n]) $($await)* ?;

                let mut rx = [0u8; 512];
                let m = self.transport.recv(&mut rx, self.config.timeout) $($await)* ?;
                if m == 0 {
                    return Err($crate::client::ClientError::Transport(
                        $crate::transport::TransportError::Disconnected,
                    ));
                }
                let response = <$adu>::decode(&rx[..m]).map_err($crate::client::ClientError::Decode)?;
                if response.transaction_id != transaction_id {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                if response.unit_id != unit_id {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                if response.pdu.is_empty() {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                Ok(response.pdu)
            }
        }
    };
}

/// Re-export the macro at the crate root so `crate::impl_adu_adapter!` works.
pub use impl_adu_adapter;

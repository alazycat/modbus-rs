//! Macros used internally by the Modbus client implementations.

/// Generate the `dispatch` method for a synchronous or asynchronous
/// [`ClientCore`](crate::client::ClientCore)/
/// [`AsyncClientCore`](crate::client::AsyncClientCore).
///
/// Call with an empty async/await pair for the synchronous implementation:
///
/// ```ignore
/// impl_dispatch!([] []);
/// ```
///
/// Call with `async`/`.await` for the asynchronous implementation:
///
/// ```ignore
/// impl_dispatch!([async] [.await]);
/// ```
#[macro_export]
macro_rules! impl_dispatch {
    ([$($async:tt)*] [$($await:tt)*]) => {
        pub $($async)* fn dispatch(
            &mut self,
            unit_id: u8,
            request_pdu: &[u8],
        ) -> Result<Vec<u8>, ClientError> {
            if request_pdu.is_empty() {
                return Err(ClientError::InvalidResponse);
            }
            let request_function = request_pdu[0];
            let response_pdu = self.adapter.send_receive(unit_id, request_pdu) $($await)* ?;
            super::validate_response_function(request_function, &response_pdu)?;
            Ok(response_pdu)
        }
    };
}

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

/// Generate typed register helper methods for a synchronous or asynchronous
/// [`ClientCore`](crate::client::ClientCore)/
/// [`AsyncClientCore`](crate::client::AsyncClientCore).
///
/// Requires the `helpers` feature. Call with an empty async/await pair for the
/// synchronous implementation:
///
/// ```ignore
/// impl_typed_helpers!([] []);
/// ```
///
/// Call with `async`/`.await` for the asynchronous implementation:
///
/// ```ignore
/// impl_typed_helpers!([async] [.await]);
/// ```
#[macro_export]
macro_rules! impl_typed_helpers {
    ([$($async:tt)*] [$($await:tt)*]) => {
        /// Read a single holding register as a `u16`.
        pub $($async)* fn read_holding_registers_u16(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<u16, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, 1) $($await)* ?;
            helpers::u16_from_bytes(&bytes, self.config.endian).map_err(ClientError::from)
        }

        /// Read a single holding register as an `i16`.
        pub $($async)* fn read_holding_registers_i16(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<i16, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, 1) $($await)* ?;
            helpers::i16_from_bytes(&bytes, self.config.endian).map_err(ClientError::from)
        }

        /// Read two holding registers as a `u32` using the configured endianness and word order.
        pub $($async)* fn read_holding_registers_u32(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<u32, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, 2) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::u32_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read two holding registers as an `i32` using the configured endianness and word order.
        pub $($async)* fn read_holding_registers_i32(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<i32, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, 2) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::i32_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read two holding registers as an `f32` using the configured endianness and word order.
        pub $($async)* fn read_holding_registers_f32(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<f32, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, 2) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::f32_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read four holding registers as a `u64` using the configured endianness and word order.
        pub $($async)* fn read_holding_registers_u64(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<u64, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, 4) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::u64_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read four holding registers as an `i64` using the configured endianness and word order.
        pub $($async)* fn read_holding_registers_i64(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<i64, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, 4) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::i64_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read four holding registers as an `f64` using the configured endianness and word order.
        pub $($async)* fn read_holding_registers_f64(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<f64, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, 4) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::f64_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read `quantity` holding registers as a NUL-terminated string.
        pub $($async)* fn read_holding_registers_string(
            &mut self,
            unit_id: u8,
            address: u16,
            quantity: u16,
        ) -> Result<String, ClientError> {
            let bytes = self.read_holding_registers(unit_id, address, quantity) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::string_from_registers(&words, self.config.endian).map_err(ClientError::from)
        }

        /// Read a single input register as a `u16`.
        pub $($async)* fn read_input_registers_u16(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<u16, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, 1) $($await)* ?;
            helpers::u16_from_bytes(&bytes, self.config.endian).map_err(ClientError::from)
        }

        /// Read a single input register as an `i16`.
        pub $($async)* fn read_input_registers_i16(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<i16, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, 1) $($await)* ?;
            helpers::i16_from_bytes(&bytes, self.config.endian).map_err(ClientError::from)
        }

        /// Read two input registers as a `u32` using the configured endianness and word order.
        pub $($async)* fn read_input_registers_u32(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<u32, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, 2) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::u32_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read two input registers as an `i32` using the configured endianness and word order.
        pub $($async)* fn read_input_registers_i32(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<i32, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, 2) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::i32_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read two input registers as an `f32` using the configured endianness and word order.
        pub $($async)* fn read_input_registers_f32(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<f32, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, 2) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::f32_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read four input registers as a `u64` using the configured endianness and word order.
        pub $($async)* fn read_input_registers_u64(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<u64, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, 4) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::u64_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read four input registers as an `i64` using the configured endianness and word order.
        pub $($async)* fn read_input_registers_i64(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<i64, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, 4) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::i64_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read four input registers as an `f64` using the configured endianness and word order.
        pub $($async)* fn read_input_registers_f64(
            &mut self,
            unit_id: u8,
            address: u16,
        ) -> Result<f64, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, 4) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::f64_from_registers(&words, self.config.endian, self.config.word_order)
                .map_err(ClientError::from)
        }

        /// Read `quantity` input registers as a NUL-terminated string.
        pub $($async)* fn read_input_registers_string(
            &mut self,
            unit_id: u8,
            address: u16,
            quantity: u16,
        ) -> Result<String, ClientError> {
            let bytes = self.read_input_registers(unit_id, address, quantity) $($await)* ?;
            let words = super::bytes_to_words(&bytes, self.config.endian)?;
            helpers::string_from_registers(&words, self.config.endian).map_err(ClientError::from)
        }

        /// Write a `u16` value to a single holding register.
        pub $($async)* fn write_multiple_registers_u16(
            &mut self,
            unit_id: u8,
            address: u16,
            value: u16,
        ) -> Result<(), ClientError> {
            self.write_registers(unit_id, address, &[value]) $($await)*
        }

        /// Write an `i16` value to a single holding register.
        pub $($async)* fn write_multiple_registers_i16(
            &mut self,
            unit_id: u8,
            address: u16,
            value: i16,
        ) -> Result<(), ClientError> {
            self.write_registers(unit_id, address, &[ value as u16]) $($await)*
        }

        /// Write a `u32` value to two holding registers using the configured endianness and word order.
        pub $($async)* fn write_multiple_registers_u32(
            &mut self,
            unit_id: u8,
            address: u16,
            value: u32,
        ) -> Result<(), ClientError> {
            let regs = helpers::u32_to_registers(value, self.config.endian, self.config.word_order);
            self.write_registers(unit_id, address, &regs) $($await)*
        }

        /// Write an `i32` value to two holding registers using the configured endianness and word order.
        pub $($async)* fn write_multiple_registers_i32(
            &mut self,
            unit_id: u8,
            address: u16,
            value: i32,
        ) -> Result<(), ClientError> {
            let regs = helpers::i32_to_registers(value, self.config.endian, self.config.word_order);
            self.write_registers(unit_id, address, &regs) $($await)*
        }

        /// Write an `f32` value to two holding registers using the configured endianness and word order.
        pub $($async)* fn write_multiple_registers_f32(
            &mut self,
            unit_id: u8,
            address: u16,
            value: f32,
        ) -> Result<(), ClientError> {
            let regs = helpers::f32_to_registers(value, self.config.endian, self.config.word_order);
            self.write_registers(unit_id, address, &regs) $($await)*
        }

        /// Write a `u64` value to four holding registers using the configured endianness and word order.
        pub $($async)* fn write_multiple_registers_u64(
            &mut self,
            unit_id: u8,
            address: u16,
            value: u64,
        ) -> Result<(), ClientError> {
            let regs = helpers::u64_to_registers(value, self.config.endian, self.config.word_order);
            self.write_registers(unit_id, address, &regs) $($await)*
        }

        /// Write an `i64` value to four holding registers using the configured endianness and word order.
        pub $($async)* fn write_multiple_registers_i64(
            &mut self,
            unit_id: u8,
            address: u16,
            value: i64,
        ) -> Result<(), ClientError> {
            let regs = helpers::i64_to_registers(value, self.config.endian, self.config.word_order);
            self.write_registers(unit_id, address, &regs) $($await)*
        }

        /// Write an `f64` value to four holding registers using the configured endianness and word order.
        pub $($async)* fn write_multiple_registers_f64(
            &mut self,
            unit_id: u8,
            address: u16,
            value: f64,
        ) -> Result<(), ClientError> {
            let regs = helpers::f64_to_registers(value, self.config.endian, self.config.word_order);
            self.write_registers(unit_id, address, &regs) $($await)*
        }

        /// Write a string to holding registers, padded to `pad_to` registers if non-zero.
        pub $($async)* fn write_multiple_registers_string(
            &mut self,
            unit_id: u8,
            address: u16,
            value: &str,
            pad_to: usize,
        ) -> Result<(), ClientError> {
            let regs = helpers::string_to_registers(value, self.config.endian, pad_to)
                .map_err(ClientError::from)?;
            self.write_registers(unit_id, address, &regs) $($await)*
        }
    };
}

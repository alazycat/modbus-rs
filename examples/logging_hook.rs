//! Example: a TCP server that logs every request and response via a hook.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example logging_hook --features sync,tcp
//! ```
//!
//! Then send Modbus TCP requests to `127.0.0.1:5502` (unit ID 1).

use modbus::exception::ExceptionResponse;
use modbus::server::{MemoryStore, RequestHook};
use modbus::tcp_server::TcpServer;

#[derive(Debug)]
struct LoggingHook;

impl RequestHook for LoggingHook {
    fn before_request(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<(), ExceptionResponse> {
        println!("[hook] unit={unit_id:02X} request={request_pdu:02X?}");
        Ok(())
    }

    fn after_response(&mut self, unit_id: u8, _request_pdu: &[u8], response_pdu: &[u8]) {
        println!("[hook] unit={unit_id:02X} response={response_pdu:02X?}");
    }
}

fn main() -> Result<(), modbus::tcp_server::TcpServerError> {
    let store = MemoryStore::new(16, 0, 4, 0);
    let mut server = TcpServer::new(store);
    server.server_mut().set_hook(Box::new(LoggingHook));

    let listener = std::net::TcpListener::bind("127.0.0.1:5502")?;
    println!("Listening on {}", listener.local_addr()?);

    for stream in listener.incoming() {
        let mut stream = stream?;
        println!("client connected");
        server.serve(&mut stream, 0x01)?;
        println!("client disconnected");
    }

    Ok(())
}

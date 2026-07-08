//! A minimal command-line Modbus client/server.
//!
//! This binary is gated by the `cli` feature and uses `clap` for argument
//! parsing and `tracing` for observability.

#![cfg(feature = "cli")]

use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::{Parser, Subcommand};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

use modbus::client::{AsyncClient, ClientConfig};
use modbus::rtu_transport::open_serial_rtu;
use modbus::exception::ExceptionCode;
use modbus::server::{DataStore, MemoryStore};
use modbus::tcp_client::{AsyncTcpClient, TcpClientConfig};
use modbus::tcp_server::AsyncTcpServer;
use modbus::tcp_transport::AsyncTcpTransport;
use modbus::AsyncServer;

#[derive(Parser)]
#[command(name = "modbus-cli")]
#[command(about = "Modbus client/server CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send requests to a Modbus server.
    Client(ClientArgs),
    /// Run a Modbus server.
    Server(ServerArgs),
}

#[derive(Parser)]
struct ClientArgs {
    #[command(subcommand)]
    transport: ClientTransport,
}

#[derive(Subcommand)]
enum ClientTransport {
    /// Connect over Modbus TCP.
    Tcp(TcpClientArgs),
    /// Connect over Modbus RTU via a serial port.
    Rtu(RtuClientArgs),
}

#[derive(Parser)]
struct TcpClientArgs {
    /// Server host.
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Server port.
    #[arg(short, long, default_value = "502")]
    port: u16,

    /// Modbus unit ID.
    #[arg(short, long, default_value = "1", value_parser = parse_u8)]
    unit_id: u8,

    /// Response timeout in seconds.
    #[arg(long, default_value = "5")]
    timeout: u64,

    #[command(subcommand)]
    op: ClientOp,
}

#[derive(Parser)]
struct RtuClientArgs {
    /// Serial port path (e.g. /dev/ttyUSB0 or COM3).
    #[arg(short, long)]
    path: String,

    /// Baud rate.
    #[arg(short, long, default_value = "9600")]
    baud: u32,

    /// Modbus slave ID.
    #[arg(short, long, default_value = "1", value_parser = parse_u8)]
    slave_id: u8,

    /// Response timeout in seconds.
    #[arg(long, default_value = "5")]
    timeout: u64,

    #[command(subcommand)]
    op: ClientOp,
}

#[derive(Subcommand)]
enum ClientOp {
    /// Read coils (FC 0x01).
    ReadCoils {
        /// Starting address.
        #[arg(short, long, value_parser = parse_u16)]
        address: u16,
        /// Number of coils.
        #[arg(short, long, value_parser = parse_u16)]
        quantity: u16,
    },
    /// Read holding registers (FC 0x03).
    ReadHoldingRegisters {
        /// Starting address.
        #[arg(short, long, value_parser = parse_u16)]
        address: u16,
        /// Number of registers.
        #[arg(short, long, value_parser = parse_u16)]
        quantity: u16,
    },
    /// Write a single coil (FC 0x05).
    WriteCoil {
        /// Coil address.
        #[arg(short, long, value_parser = parse_u16)]
        address: u16,
        /// Coil value.
        #[arg(short, long)]
        value: bool,
    },
    /// Write a single holding register (FC 0x06).
    WriteRegister {
        /// Register address.
        #[arg(short, long, value_parser = parse_u16)]
        address: u16,
        /// Register value.
        #[arg(short, long, value_parser = parse_u16)]
        value: u16,
    },
}

#[derive(Parser)]
struct ServerArgs {
    #[command(subcommand)]
    transport: ServerTransport,
}

#[derive(Subcommand)]
enum ServerTransport {
    /// Listen on Modbus TCP.
    Tcp(TcpServerArgs),
    /// Listen on Modbus RTU via a serial port.
    Rtu(RtuServerArgs),
}

#[derive(Parser)]
struct TcpServerArgs {
    /// Bind address.
    #[arg(short, long, default_value = "127.0.0.1:502")]
    bind: SocketAddr,

    /// Modbus unit ID.
    #[arg(short, long, default_value = "1", value_parser = parse_u8)]
    unit_id: u8,

    /// Number of coils.
    #[arg(long, default_value = "0", value_parser = parse_u16)]
    coils: u16,

    /// Number of discrete inputs.
    #[arg(long, default_value = "0", value_parser = parse_u16)]
    discrete_inputs: u16,

    /// Number of holding registers.
    #[arg(long, default_value = "0", value_parser = parse_u16)]
    holding_registers: u16,

    /// Number of input registers.
    #[arg(long, default_value = "0", value_parser = parse_u16)]
    input_registers: u16,
}

#[derive(Parser)]
struct RtuServerArgs {
    /// Serial port path (e.g. /dev/ttyUSB0 or COM3).
    #[arg(short, long)]
    path: String,

    /// Baud rate.
    #[arg(short, long, default_value = "9600")]
    baud: u32,

    /// Modbus slave ID.
    #[arg(short, long, default_value = "1", value_parser = parse_u8)]
    slave_id: u8,

    /// Number of coils.
    #[arg(long, default_value = "0", value_parser = parse_u16)]
    coils: u16,

    /// Number of discrete inputs.
    #[arg(long, default_value = "0", value_parser = parse_u16)]
    discrete_inputs: u16,

    /// Number of holding registers.
    #[arg(long, default_value = "0", value_parser = parse_u16)]
    holding_registers: u16,

    /// Number of input registers.
    #[arg(long, default_value = "0", value_parser = parse_u16)]
    input_registers: u16,
}

fn parse_u8(s: &str) -> Result<u8, clap::Error> {
    parse(s, |v| u8::from_str_radix(v, 16), |v| v.parse::<u8>())
}

fn parse_u16(s: &str) -> Result<u16, clap::Error> {
    parse(s, |v| u16::from_str_radix(v, 16), |v| v.parse::<u16>())
}

fn parse<T>(
    s: &str,
    hex: impl FnOnce(&str) -> Result<T, std::num::ParseIntError>,
    dec: impl FnOnce(&str) -> Result<T, std::num::ParseIntError>,
) -> Result<T, clap::Error> {
    let result = if let Some(v) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        hex(v)
    } else {
        dec(s)
    };
    result.map_err(|e| {
        clap::Error::raw(
            clap::error::ErrorKind::InvalidValue,
            format!("invalid integer '{s}': {e}"),
        )
    })
}

#[derive(Clone)]
struct SharedStore(Arc<Mutex<MemoryStore>>);

impl SharedStore {
    fn new(store: MemoryStore) -> Self {
        Self(Arc::new(Mutex::new(store)))
    }
}

impl DataStore for SharedStore {
    fn read_coils(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        self.0.lock().unwrap().read_coils(address, quantity)
    }

    fn read_discrete_inputs(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        self.0.lock().unwrap().read_discrete_inputs(address, quantity)
    }

    fn read_holding_registers(
        &self,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ExceptionCode> {
        self.0.lock().unwrap().read_holding_registers(address, quantity)
    }

    fn read_input_registers(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        self.0.lock().unwrap().read_input_registers(address, quantity)
    }

    fn write_coil(&mut self, address: u16, value: bool) -> Result<(), ExceptionCode> {
        self.0.lock().unwrap().write_coil(address, value)
    }

    fn write_register(&mut self, address: u16, value: u16) -> Result<(), ExceptionCode> {
        self.0.lock().unwrap().write_register(address, value)
    }

    fn write_coils(&mut self, address: u16, values: &[bool]) -> Result<(), ExceptionCode> {
        self.0.lock().unwrap().write_coils(address, values)
    }

    fn write_registers(&mut self, address: u16, values: &[u16]) -> Result<(), ExceptionCode> {
        self.0.lock().unwrap().write_registers(address, values)
    }

    fn read_exception_status(&self) -> Result<u8, ExceptionCode> {
        self.0.lock().unwrap().read_exception_status()
    }

    fn diagnostics(
        &mut self,
        sub_function: u16,
        data: u16,
    ) -> Result<(u16, u16), ExceptionCode> {
        self.0.lock().unwrap().diagnostics(sub_function, data)
    }

    fn get_comm_event_counter(&self) -> Result<(u16, u16), ExceptionCode> {
        self.0.lock().unwrap().get_comm_event_counter()
    }

    fn get_comm_event_log(&self) -> Result<(u16, u16, u16, Vec<u8>), ExceptionCode> {
        self.0.lock().unwrap().get_comm_event_log()
    }

    fn report_server_id(&self) -> Result<Vec<u8>, ExceptionCode> {
        self.0.lock().unwrap().report_server_id()
    }

    fn read_fifo_queue(&self, fifo_pointer_address: u16) -> Result<(u16, Vec<u8>), ExceptionCode> {
        self.0.lock().unwrap().read_fifo_queue(fifo_pointer_address)
    }

    fn read_file_record(
        &self,
        sub_requests: &[modbus::ReadFileRecordSubRequest],
    ) -> Result<Vec<modbus::ReadFileRecordSubResponse>, ExceptionCode> {
        self.0.lock().unwrap().read_file_record(sub_requests)
    }

    fn write_file_record(
        &mut self,
        sub_requests: &[modbus::WriteFileRecordSubRequest],
    ) -> Result<Vec<modbus::WriteFileRecordSubResponse>, ExceptionCode> {
        self.0.lock().unwrap().write_file_record(sub_requests)
    }

    fn encapsulated_interface_transport(
        &self,
        mei_type: u8,
        data: &[u8],
    ) -> Result<(u8, Vec<u8>), ExceptionCode> {
        self.0
            .lock()
            .unwrap()
            .encapsulated_interface_transport(mei_type, data)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Client(args) => run_client(args).await,
        Commands::Server(args) => run_server(args).await,
    }
}

async fn run_client(args: ClientArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match args.transport {
        ClientTransport::Tcp(tcp) => {
            let addr: SocketAddr = format!("{}:{}", tcp.host, tcp.port).parse()?;
            info!("connecting to {}:{}", tcp.host, tcp.port);
            let stream = TcpStream::connect(addr).await?;
            let config = TcpClientConfig {
                timeout: Duration::from_secs(tcp.timeout),
                ..Default::default()
            };
            let mut client = AsyncTcpClient::with_config(AsyncTcpTransport::new(stream), config);
            execute_client_op(&mut client, tcp.unit_id, tcp.op).await?;
        }
        ClientTransport::Rtu(rtu) => {
            info!("opening serial port {} at {} baud", rtu.path, rtu.baud);
            let transport = open_serial_rtu(&rtu.path, rtu.baud).await?;
            let config = ClientConfig {
                timeout: Duration::from_secs(rtu.timeout),
                ..Default::default()
            };
            let mut client = AsyncClient::with_config(transport, config);
            execute_client_op(&mut client, rtu.slave_id, rtu.op).await?;
        }
    }
    Ok(())
}

async fn execute_client_op<C, E>(
    client: &mut C,
    unit_id: u8,
    op: ClientOp,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    C: ClientMethods<E>,
    E: fmt::Debug + fmt::Display + Send + Sync + std::error::Error + 'static,
{
    match op {
        ClientOp::ReadCoils { address, quantity } => {
            let bytes = client.read_coils(unit_id, address, quantity).await?;
            println!("{}", format_hex(&bytes));
        }
        ClientOp::ReadHoldingRegisters { address, quantity } => {
            let bytes = client.read_holding_registers(unit_id, address, quantity).await?;
            println!("{}", format_hex(&bytes));
        }
        ClientOp::WriteCoil { address, value } => {
            client.write_coil(unit_id, address, value).await?;
            println!("OK");
        }
        ClientOp::WriteRegister { address, value } => {
            client.write_register(unit_id, address, value).await?;
            println!("OK");
        }
    }
    Ok(())
}

trait ClientMethods<E: std::error::Error> {
    async fn read_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, E>;
    async fn read_holding_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, E>;
    async fn write_coil(&mut self, unit_id: u8, address: u16, value: bool) -> Result<(), E>;
    async fn write_register(&mut self, unit_id: u8, address: u16, value: u16) -> Result<(), E>;
}

impl<T> ClientMethods<modbus::tcp_client::TcpClientError> for AsyncTcpClient<T>
where
    T: modbus::transport::AsyncTransport + Send,
{
    async fn read_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, modbus::tcp_client::TcpClientError> {
        (**self).read_coils(unit_id, address, quantity).await
    }
    async fn read_holding_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, modbus::tcp_client::TcpClientError> {
        (**self).read_holding_registers(unit_id, address, quantity).await
    }
    async fn write_coil(
        &mut self,
        unit_id: u8,
        address: u16,
        value: bool,
    ) -> Result<(), modbus::tcp_client::TcpClientError> {
        (**self).write_coil(unit_id, address, value).await
    }
    async fn write_register(
        &mut self,
        unit_id: u8,
        address: u16,
        value: u16,
    ) -> Result<(), modbus::tcp_client::TcpClientError> {
        (**self).write_register(unit_id, address, value).await
    }
}

impl<T> ClientMethods<modbus::client::ClientError> for AsyncClient<T>
where
    T: modbus::transport::AsyncTransport + Send,
{
    async fn read_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, modbus::client::ClientError> {
        (**self).read_coils(unit_id, address, quantity).await
    }
    async fn read_holding_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, modbus::client::ClientError> {
        (**self).read_holding_registers(unit_id, address, quantity).await
    }
    async fn write_coil(
        &mut self,
        unit_id: u8,
        address: u16,
        value: bool,
    ) -> Result<(), modbus::client::ClientError> {
        (**self).write_coil(unit_id, address, value).await
    }
    async fn write_register(
        &mut self,
        unit_id: u8,
        address: u16,
        value: u16,
    ) -> Result<(), modbus::client::ClientError> {
        (**self).write_register(unit_id, address, value).await
    }
}

async fn run_server(args: ServerArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match args.transport {
        ServerTransport::Tcp(tcp) => run_tcp_server(tcp).await,
        ServerTransport::Rtu(rtu) => run_rtu_server(rtu).await,
    }
}

async fn run_tcp_server(
    args: TcpServerArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let store = SharedStore::new(MemoryStore::new(
        args.coils,
        args.discrete_inputs,
        args.holding_registers,
        args.input_registers,
    ));

    let listener = TcpListener::bind(args.bind).await?;
    info!("TCP server listening on {}", args.bind);

    loop {
        let (mut stream, peer) = listener.accept().await?;
        let mut server = AsyncTcpServer::new(store.clone());
        info!("accepted connection from {}", peer);
        tokio::spawn(async move {
            if let Err(e) = server.serve(&mut stream, args.unit_id).await {
                error!("connection from {} failed: {}", peer, e);
            }
        });
    }
}

async fn run_rtu_server(
    args: RtuServerArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let store = SharedStore::new(MemoryStore::new(
        args.coils,
        args.discrete_inputs,
        args.holding_registers,
        args.input_registers,
    ));

    info!("opening serial port {} at {} baud", args.path, args.baud);
    let transport = open_serial_rtu(&args.path, args.baud).await?;
    let mut stream = transport.into_inner();
    let mut server = AsyncServer::new(store);

    info!("RTU server serving slave {}", args.slave_id);
    server.serve(&mut stream, args.slave_id).await?;
    Ok(())
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

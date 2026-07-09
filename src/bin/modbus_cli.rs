//! A minimal command-line Modbus client/server.
//!
//! This binary is gated by the `cli` feature and uses `clap` for argument
//! parsing and `tracing` for observability.

#![cfg(feature = "cli")]

use std::fmt;
use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::path::PathBuf;
#[cfg(feature = "tls")]
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

use modbus::client::{AsyncClient, ClientConfig};
use modbus::rtu_transport::open_serial_rtu;
use modbus::server::{MemoryStore, SharedStore};
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

    /// Enable TLS.
    #[cfg(feature = "tls")]
    #[arg(long)]
    tls: bool,

    /// Path to PEM-encoded TLS certificate.
    /// For servers this is the server certificate; for clients it is the
    /// trusted server/CA certificate.
    #[cfg(feature = "tls")]
    #[arg(long, value_name = "PATH")]
    tls_cert: Option<PathBuf>,

    /// Path to PEM-encoded TLS private key (server only).
    #[cfg(feature = "tls")]
    #[arg(long, value_name = "PATH")]
    tls_key: Option<PathBuf>,

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

    /// Enable TLS.
    #[cfg(feature = "tls")]
    #[arg(long)]
    tls: bool,

    /// Path to PEM-encoded TLS certificate (server only).
    #[cfg(feature = "tls")]
    #[arg(long, value_name = "PATH")]
    tls_cert: Option<PathBuf>,

    /// Path to PEM-encoded TLS private key (server only).
    #[cfg(feature = "tls")]
    #[arg(long, value_name = "PATH")]
    tls_key: Option<PathBuf>,
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
            let config = TcpClientConfig {
                timeout: Duration::from_secs(tcp.timeout),
                ..Default::default()
            };
            #[cfg(feature = "tls")]
            if tcp.tls {
                let cert_path = tcp
                    .tls_cert
                    .as_ref()
                    .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                        "--tls requires --tls-cert for the trusted server/CA certificate".into()
                    })?;
                let connector = build_tls_connector(cert_path)?;
                let mut client = AsyncTcpClient::connect_tls(addr, &tcp.host, connector, config).await?;
                execute_client_op(&mut client, tcp.unit_id, tcp.op).await?;
            } else {
                let stream = TcpStream::connect(addr).await?;
                let mut client = AsyncTcpClient::with_config(AsyncTcpTransport::new(stream), config);
                execute_client_op(&mut client, tcp.unit_id, tcp.op).await?;
            }
            #[cfg(not(feature = "tls"))]
            {
                let stream = TcpStream::connect(addr).await?;
                let mut client = AsyncTcpClient::with_config(AsyncTcpTransport::new(stream), config);
                execute_client_op(&mut client, tcp.unit_id, tcp.op).await?;
            }
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
            let bytes = client
                .read_holding_registers(unit_id, address, quantity)
                .await?;
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
    async fn read_coils(&mut self, unit_id: u8, address: u16, quantity: u16) -> Result<Vec<u8>, E>;
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
        (**self)
            .read_holding_registers(unit_id, address, quantity)
            .await
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
        (**self)
            .read_holding_registers(unit_id, address, quantity)
            .await
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

    #[cfg(feature = "tls")]
    if args.tls {
        let cert_path = args
            .tls_cert
            .as_ref()
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                "--tls requires --tls-cert".into()
            })?;
        let key_path = args
            .tls_key
            .as_ref()
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                "--tls requires --tls-key".into()
            })?;
        let tls_config = build_tls_server_config(cert_path, key_path)?;
        info!("TLS TCP server listening on {}", args.bind);
        let mut server = AsyncTcpServer::new(store);
        return server.serve_tls(listener, args.unit_id, tls_config).await.map_err(Into::into);
    }

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

#[cfg(feature = "tls")]
fn load_certs(path: &PathBuf) -> Result<Vec<tokio_rustls::rustls::pki_types::CertificateDer<'static>>, Box<dyn std::error::Error + Send + Sync>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("failed to parse certificate from {}: {e}", path.display()))?;
    if certs.is_empty() {
        return Err(format!("no certificates found in {}", path.display()).into());
    }
    Ok(certs)
}

#[cfg(feature = "tls")]
fn load_private_key(path: &PathBuf) -> Result<tokio_rustls::rustls::pki_types::PrivateKeyDer<'static>, Box<dyn std::error::Error + Send + Sync>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| format!("failed to parse private key from {}: {e}", path.display()))?
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            format!("no private key found in {}", path.display()).into()
        })
}

#[cfg(feature = "tls")]
fn build_tls_connector(
    cert_path: &PathBuf,
) -> Result<tokio_rustls::TlsConnector, Box<dyn std::error::Error + Send + Sync>> {
    let certs = load_certs(cert_path)?;
    let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
    for cert in certs {
        root_store.add(cert)?;
    }
    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(tokio_rustls::TlsConnector::from(Arc::new(config)))
}

#[cfg(feature = "tls")]
fn build_tls_server_config(
    cert_path: &PathBuf,
    key_path: &PathBuf,
) -> Result<Arc<tokio_rustls::rustls::ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
    let cert_chain = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| format!("failed to build TLS server config: {e}"))?;
    Ok(Arc::new(config))
}

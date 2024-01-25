use clap::Parser;
use anyhow::Result;
use handler::Handler;
use hickory_server::ServerFuture;
use tokio::net::UdpSocket;
use std::net::SocketAddr;

mod handler;
mod parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Domain name
    #[clap(long, short, default_value = "octet.zone", env = "OCTETZONE_DOMAIN")]
    pub domain: String,

	/// UDP socket to listen on.
    #[clap(long, short, default_value = "0.0.0.0:1053", env = "OCTETZONE_LISTEN")]
    pub udp: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let args = Args::parse();
	let handler = Handler::from_options(&args);

	let mut server = ServerFuture::new(handler);

	server.register_socket(UdpSocket::bind(args.udp).await?);

	server.block_until_done().await?;

	Ok(())
}

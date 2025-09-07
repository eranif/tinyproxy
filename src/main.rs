use clap::Parser;
use log::{debug, error, info, warn};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Source address:port to listen on (e.g., "[IP_ADDRESS]")
    #[arg(short, long)]
    source: String,

    /// Destination address:port to forward to (e.g., "[IP_ADDRESS]")
    #[arg(short, long)]
    destination: String,
}

fn handle_connection(mut client: TcpStream, destination_addr: &str) -> io::Result<()> {
    let client_addr = client.peer_addr()?;
    info!("New connection from {}", client_addr);

    // Connect to destination
    let mut server = TcpStream::connect(destination_addr)?;
    info!("Connected to destination {}", destination_addr);

    // Clone streams for bidirectional communication
    let mut client_read = client.try_clone()?;
    let mut server_read = server.try_clone()?;

    // Forward client -> server
    let t1 = thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match client_read.read(&mut buffer) {
                Ok(0) => {
                    info!("Client {} closed connection", client_addr);
                    break;
                }
                Ok(n) => {
                    debug!("<< {}", String::from_utf8_lossy(&buffer));
                    if server.write_all(&buffer[..n]).is_err() {
                        warn!("Failed to write to server");
                        break;
                    }
                }
                Err(e) => {
                    warn!("Error reading from client {}: {}", client_addr, e);
                    break;
                }
            }
        }
    });

    // Forward server -> client
    let client_addr_clone = client_addr;
    let t2 = thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match server_read.read(&mut buffer) {
                Ok(0) => {
                    info!("Server closed connection for client {}", client_addr_clone);
                    break;
                }
                Ok(n) => {
                    debug!(">> {}", String::from_utf8_lossy(&buffer));
                    if client.write_all(&buffer[..n]).is_err() {
                        warn!("Failed to write to client {}", client_addr_clone);
                        break;
                    }
                }
                Err(e) => {
                    warn!(
                        "Error reading from server for client {}: {}",
                        client_addr_clone, e
                    );
                    break;
                }
            }
        }
    });

    // Wait for both directions to complete
    t1.join().unwrap();
    t2.join().unwrap();

    info!("Connection closed for {}", client_addr);
    Ok(())
}

fn main() -> io::Result<()> {
    // Initialize the logger
    env_logger::init();

    let args = Args::parse();

    let listener = TcpListener::bind(&args.source)?;
    info!("Proxy listening on {}", args.source);
    info!("Forwarding to {}", args.destination);

    for stream in listener.incoming() {
        match stream {
            Ok(client) => {
                let dest_addr = args.destination.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_connection(client, &dest_addr) {
                        error!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }

    Ok(())
}

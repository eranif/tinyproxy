use clap::Parser;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Source address:port to listen on (e.g., "0.0.0.0:1234")
    #[arg(short, long)]
    source: String,

    /// Destination address:port to forward to (e.g., "127.0.0.1:5678")
    #[arg(short, long)]
    destination: String,
}

fn handle_connection(mut client: TcpStream, destination_addr: &str) -> io::Result<()> {
    // Connect to destination
    let mut server = TcpStream::connect(destination_addr)?;

    // Clone streams for bidirectional communication
    let mut client_read = client.try_clone()?;
    let mut server_read = server.try_clone()?;

    // Forward client -> server
    let t1 = thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match client_read.read(&mut buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    print!("<== {}", String::from_utf8_lossy(&buffer));
                    if server.write_all(&buffer[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Forward server -> client
    let t2 = thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match server_read.read(&mut buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    print!("==> {}", String::from_utf8_lossy(&buffer));
                    if client.write_all(&buffer[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Wait for both directions to complete
    t1.join().unwrap();
    t2.join().unwrap();

    Ok(())
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let listener = TcpListener::bind(&args.source)?;
    println!("Proxy listening on {}", args.source);
    println!("Forwarding to {}", args.destination);

    for stream in listener.incoming() {
        match stream {
            Ok(client) => {
                let dest_addr = args.destination.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_connection(client, &dest_addr) {
                        eprintln!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }
    }

    Ok(())
}

use clap::Parser;
use cli::common::split_stream;
use log::{error, info};
use sindri::api::Api;
use sindri::common::jobs::{Request, Response};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Unix domain socket path
    #[clap(short, long, default_value = "sindri.sock")]
    socket: PathBuf,
}

fn main() {
    simple_logger::SimpleLogger::new()
        .init()
        .expect("Failed to initialize logger");
    let args = Args::parse();

    // Connect to socket
    let stream = UnixStream::connect(&args.socket).expect("Failed to connect to socket");
    info!("Connected to '{}'", args.socket.to_string_lossy());

    // Instantiate API
    let (sender, receiver) = split_stream(0, stream);
    let mut api = Api { sender, receiver };

    // Send request
    let request = Request::GetRandom { size: 16 };
    info!("Sending request");
    api.enqueue(request).expect("Failed to enqueue request");
    info!("Receiving response");
    let response = api.dequeue().expect("Failed to dequeue response");
    info!("Received response");
    match response {
        Response::Error(e) => {
            error!("Response: Error: {:?}", e)
        }
        Response::GetRandom { data } => {
            info!(
                "Response: random data: ({} bytes): {}",
                data.len(),
                hex::encode(data)
            )
        }
    }
}
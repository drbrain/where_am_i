use crate::JsonSender;
use crate::JsonReceiver;

use std::net::SocketAddr;

use tokio::io::BufWriter;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;

use tracing::error;
use tracing::debug;
use tracing::info;

#[tracing::instrument]
pub async fn spawn(port: u16, tx: JsonSender) {
    let address = ("0.0.0.0", port);

    let mut listener = TcpListener::bind(address).await.unwrap();

    loop {
        info!("waiting for client");

        let (socket, addr) = listener.accept().await.unwrap();

        info!("client connected {:?}", addr);

        tokio::spawn(handle_client(socket, tx.subscribe()));
    }
}

#[tracing::instrument]
async fn handle_client(mut socket: TcpStream, mut rx: JsonReceiver) {
    let (_recv, send) = socket.split();

    let mut send = BufWriter::new(send);

    loop {
        let json = match rx.recv().await {
            Ok(j) => j,
            Err(e) => {
                error!("error receiving message: {:?}", e);
                continue;
            },
        };

        let message = format!("{}\n", json);

        send.write(message.as_bytes()).await.unwrap();

        match send.flush().await {
            Ok(()) => (),
            Err(e) => {
                break;
            },
        }
    }

    info!("client disconnected");
}

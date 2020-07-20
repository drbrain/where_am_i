use crate::JsonSender;

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

        tokio::spawn(handle_client(socket, tx.clone())).await.unwrap();
    }
}

#[tracing::instrument]
async fn handle_client(mut socket: TcpStream, tx: JsonSender) {
    let (_recv, send) = socket.split();
    let mut rx = tx.subscribe();

    let mut send = BufWriter::new(send);

    loop {
        debug!("waiting for message from {:?}", rx);
        let json = match rx.recv().await {
            Ok(j) => j,
            Err(e) => {
                error!("error receiving message: {:?}", e);
                continue;
            },
        };

        let message = format!("{}\n", json);

        send.write(message.as_bytes()).await.unwrap();

        send.flush().await.unwrap();
    }
}

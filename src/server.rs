use crate::JsonQueue;

use json::parse;
use json::stringify;

use tokio::io::BufReader;
use tokio::io::BufWriter;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::net::tcp::WriteHalf;
use tokio::prelude::*;
use tokio::sync::broadcast;

pub fn spawn(port: u16) -> JsonQueue {
    let (tx, _) = broadcast::channel(5);
    let time_tx = tx.clone();

    let address = ("0.0.0.0", port);

    tokio::spawn(async move {
        let mut listener = TcpListener::bind(address).await.unwrap();

        loop {
            let (socket, _) = listener.accept().await.unwrap();

            let nodelay = socket.set_nodelay(true);

            if nodelay.is_err() {
                continue;
            }

            handle_client(socket, time_tx.clone()).await;
        }
    });

    return tx;
}

async fn handle_client(mut socket: TcpStream, tx: JsonQueue) {
    let (recv, send) = socket.split();

    let recv = BufReader::new(recv);
    let mut lines = recv.lines();

    let mut send = BufWriter::new(send);

    loop {
        let request = match lines.next_line().await {
            Ok(l) => l,
            Err(_) => break,
        };

        if request.is_none() {
            break;
        }

        let request = request.unwrap();

        if request == "?VERSION;".to_string() {
            let version = json::object!{
                class: "VERSION",
                release: "where_am_i 0.0.0",
                rev: "",
                proto_major: 3,
                proto_minor: 10,
            };

            let message = format!("{}\n", stringify(version));

            let result = send.write(message.as_bytes()).await;

            if result.is_err() {
                break;
            }
        } else if request.starts_with("?WATCH=") {
            let json_start = match request.find("{") {
                Some(i) => i,
                None    => continue,
            };

            let json_end = match request.rfind("}") {
                Some(i) => i,
                None    => continue,
            };

            let watch_json = match request.get(json_start..=json_end) {
                Some(j) => j,
                None    => continue,
            };

            let watch = match parse(watch_json) {
                Ok(w) => w,
                Err(_) => {
                    eprintln!("Error parsing WATCH body {}", watch_json);
                    continue;
                },
            };

            let _device = watch["device"].as_str().unwrap_or_else(|| "UNSET");

            let enable = watch["enable"].as_bool().unwrap_or_else(|| false);

            let pps = watch["pps"].as_bool().unwrap_or_else(|| false);

            if enable && pps {
                relay_time_messages(&mut send, tx.clone()).await;
            }
        }

        let flushed = send.flush().await;
        if flushed.is_err() {
            break;
        }
    }
}

async fn relay_time_messages(send: &mut BufWriter<WriteHalf<'_>>, tx: JsonQueue) {
    let mut rx = tx.subscribe();

    loop {
        let toff = match rx.recv().await {
            Ok(t) => t,
            Err(_) => break,
        };

        let message = format!("{}\n", stringify(toff));

        match send.write(message.as_bytes()).await {
            Ok(_) => (),
            Err(_) => break,
        };

        let flushed = send.flush().await;
        if flushed.is_err() {
            break;
        }
    }
}


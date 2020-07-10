extern crate json;

use json::parse;
use json::stringify;

use nmea::Nmea;
use nmea::ParseResult;

use std::env;

use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::BufWriter;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    let input = open_socket().await;

    let nmea_tx = spawn_server(2947);

    let (lines_rx, done_rx) = spawn_reader(input, nmea_tx);

    spawn_parser(lines_rx);

    done_rx.await.unwrap();
}

async fn open_socket() -> BufReader<File> {
    let name = env::args().nth(1);

    if name.is_none() {
        println!("Provide GPS device as first argument");
        std::process::exit(1);
    }

    let name = name.unwrap();

    let io = match File::open(name).await {
        Ok(io) => io,
        Err(e) => {
            println!("Error {}", e);
            std::process::exit(1);
        }
    };

    let input = BufReader::new(io);

    return input;
}

async fn handle_client(mut socket: TcpStream, nmea_tx: broadcast::Sender<String>) {
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

        println!("{:?}", request);

        if request == "?VERSION;".to_string() {
            let version = json::object!{
                class: "VERSION",
                release: "where_am_i 0.0.0",
                rev: "",
                proto_major: 3,
                proto_minor: 10,
            };

            let mut version_json = stringify(version);
            println!("{:?}", version_json);
            version_json.push('\n');

            let result = send.write(version_json.as_bytes()).await;

            println!("{:?}", result);

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
                    println!("Error parsing WATCH body {}", watch_json);
                    continue;
                },
            };

            let device = watch["device"].as_str().unwrap_or_else(|| "UNSET");

            let enable = watch["enable"].as_bool().unwrap_or_else(|| false);

            let pps = watch["pps"].as_bool().unwrap_or_else(|| false);

            println!("device: {} enabled: {} pps: {}", device, enable, pps);

            if enable && pps {
                let mut rx = nmea_tx.subscribe();

                loop {
                    let mut message = match rx.recv().await {
                        Ok(m) => m,
                        Err(_) => break,
                    };

                    message.push('\n');

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
        }

        let flushed = send.flush().await;
        if flushed.is_err() {
            break;
        }
    }
}

fn spawn_server(port: u16) -> broadcast::Sender<String> {
    let (tx, _) = broadcast::channel(5);
    let nmea_tx = tx.clone();

    let address = ("0.0.0.0", port);

    tokio::spawn(async move {
        let mut listener = TcpListener::bind(address).await.unwrap();

        loop {
            let (socket, _) = listener.accept().await.unwrap();

            let nodelay = socket.set_nodelay(true);

            if nodelay.is_err() {
                continue;
            }

            handle_client(socket, nmea_tx.clone()).await;
        }
    });

    return tx;
}

fn spawn_parser(mut lines: mpsc::Receiver<String>) {
    let mut nmea = Nmea::new();

    tokio::spawn(async move {
        while let Some(line) = lines.recv().await {
            let result = nmea.parse(&line.to_string());

            match result {
                Ok(s) => {
                    match s {
                        ParseResult::GGA(gga) => {
                            println!("{:?}", gga.fix_time);
                        },
                        _ => ()
                    }
                },
                Err(_) => (),
            }
        }
    });
}

fn spawn_reader(input: BufReader<File>, nmea_tx: broadcast::Sender<String>) -> (mpsc::Receiver<String>, oneshot::Receiver<bool>) {
    let (mut lines_tx, lines_rx) = mpsc::channel(5);
    let (done_tx, done_rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut lines = input.lines();

        loop {
            let result = lines.next_line().await;

            let line = match result {
                Ok(line) => line,
                Err(_)   => {
                    eprintln!("GPS disconnected");
                    std::process::exit(1);
                }
            };

            let line = match line {
                Some(line) => line,
                None => {
                    done_tx.send(false).unwrap();
                    break;
                }
            };

            match nmea_tx.send(line.clone()) {
                Ok(_)  => (),
                Err(_) => (),
            };

            lines_tx.send(line.clone()).await.unwrap();
        }
    });

    return (lines_rx, done_rx);
}

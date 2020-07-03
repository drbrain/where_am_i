use nmea::Nmea;
use nmea::SentenceType;

use std::env;

use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
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

async fn send_to_client(mut socket: TcpStream, mut nmea_rx: broadcast::Receiver<String>) {
    loop {
        let mut line = nmea_rx.recv().await.unwrap();

        line.push('\n');

        let result = socket.write(line.as_bytes()).await;

        match result {
            Ok(_)  => (),
            Err(_) => break,
        };
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
            let nmea_rx = nmea_tx.subscribe();

            send_to_client(socket, nmea_rx).await;
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
                Ok(sentence) => {
                    match sentence {
                        SentenceType::GGA => println!("{:?}", sentence),
                        _ => ()
                    }
                },
                Err(error) => println!("E: {}", error),
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
                Err(_)   => std::process::exit(1),
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

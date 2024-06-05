use std::error::Error;
use std::io::{prelude::*, stdin, stdout, BufReader};
use std::net::{IpAddr, TcpStream};
use std::str::FromStr;
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::{env, str};
use std::{thread, time};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use local_ip_address::local_ip;

const SLEEP_LENGTH: time::Duration = time::Duration::from_millis(100);

async fn stream_io_thread(mut stream: TcpStream, other_usr: String) -> Sender<String> {
    let mut reader = BufReader::new(stream.try_clone().expect("failed to clone stream."));
    let (input_sender, input_receiver) = channel::<String>();
    thread::spawn(move || loop {
        let mut line = String::new();
        if let Ok(_err) = reader.read_line(&mut line) {
            if line != "" {
                println!("{}: {}", other_usr, line.trim());
            }
        }
        match input_receiver.try_recv() {
            Ok(usr_input) => {
                match stream.write(usr_input.as_bytes()) {
                    Err(_e) => return, // end thread
                    Ok(f) => f,
                };
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        }
        thread::sleep(SLEEP_LENGTH);
    });
    input_sender
}

async fn chat(stream: TcpStream, usr: &str) {
    println!("Entering chat...");
    stream.set_nonblocking(true).unwrap();
    let input_sender = stream_io_thread(stream, usr.to_string()).await;
    loop {
        thread::sleep(SLEEP_LENGTH);
        let mut usr_input = String::new();
        stdin()
            .read_line(&mut usr_input)
            .expect("Failed to read from stdin.");
        match input_sender.send(usr_input) {
            Err(_e) => return,
            Ok(f) => f,
        };
    }
}

async fn connect(usr_name: String, addr: String) -> std::io::Result<()> {
    println!("Connecting...");
    let mut stream = TcpStream::connect(addr)?;
    stream.write(usr_name.as_bytes())?;
    let mut line = [0; 128];
    stream.read(&mut line)?;
    chat(stream, str::from_utf8(&line).unwrap()).await;
    Ok(())
}

async fn listen(usr_name: String, port: usize) -> std::io::Result<()> {
    let addr = local_ip().unwrap_or(IpAddr::from_str("127.0.0.1").unwrap());
    let listener = TcpListener::bind(format!("{addr}:{port}")).await?;

    let mut stream = listener.accept().await?.0;
    stream.write(usr_name.as_bytes()).await?;
    let mut line = [0; 128];
    stream.read(&mut line).await?;
    chat(stream.into_std()?, str::from_utf8(&line).unwrap()).await;

    Ok(())
}

fn get_usr_name() -> String {
    print!("Enter your name: ");
    stdout().flush().expect("unable to flush stdout");
    let mut usr_name = String::new();
    stdin()
        .read_line(&mut usr_name)
        .expect("Unable to read stdin.");
    usr_name.trim().to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if args[1] == "server" {
            println!("I'm a server");
            if let Err(err) = listen(
                get_usr_name(),
                args.last()
                    .unwrap_or(&"8888".to_string())
                    .parse()
                    .unwrap_or(8888),
            )
            .await
            {
                println!("an error occured in listen(): {err}");
            }
        } else if args[1] == "client" {
            println!("I'm a client");
            match connect(
                get_usr_name(),
                args.last()
                    .unwrap_or(&"localhost:8888".to_string())
                    .to_string(),
            )
            .await
            {
                Err(_e) => println!("There is no server listening for connections."),
                Ok(_f) => println!("Connection ended."),
            };
        }
    }
    Ok(())
}

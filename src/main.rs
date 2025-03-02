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

use crossterm::{cursor, queue};

const SLEEP_LENGTH: time::Duration = time::Duration::from_millis(100);

async fn stream_io_thread(mut stream: TcpStream, other_usr: String) -> Sender<String> {
    let mut reader = BufReader::new(stream.try_clone().expect("Failed to clone stream!"));
    let (input_sender, input_receiver) = channel::<String>();
    thread::spawn(move || loop {
        let mut line = String::new();
        if let Ok(_err) = reader.read_line(&mut line) {
            if !line.is_empty() {
                let message = format!(
                    "\x1b[93m{}\x1b[0m: {}",
                    other_usr.trim_matches('\0'),
                    line.trim()
                );
                let mut stdout = stdout();
                queue!(stdout, cursor::MoveUp(1)).unwrap();
                println!("\r\n{message}");
                queue!(stdout, cursor::MoveDown(2)).unwrap();
                println!();
                print!("\x1b[96m> ");
                stdout.flush().unwrap();
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
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected!"),
        }
        thread::sleep(SLEEP_LENGTH);
    });
    input_sender
}

async fn chat(stream: TcpStream, usr: &str) {
    println!("Entering chat...\n");
    stream.set_nonblocking(true).unwrap();
    let input_sender = stream_io_thread(stream, usr.to_string()).await;
    let mut stdout = stdout();
    queue!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )
    .unwrap();
    queue!(stdout, cursor::MoveToNextLine(100)).unwrap();
    loop {
        thread::sleep(SLEEP_LENGTH);
        let usr_input = take_input();
        if usr_input == "q!\n" {
            return;
        }
        if usr_input == "\n" {
            continue;
        }
        println!();
        match input_sender.send(usr_input) {
            Err(_e) => return,
            Ok(f) => f,
        };
    }
}

fn take_input() -> String {
    print!("\x1b[96m> ");
    stdout().flush().expect("Failed to flush stdout!");

    let mut input: String = String::new();
    stdin()
        .read_line(&mut input)
        .expect("Failed to read from stdin!");
    print!("\x1b[0m");
    stdout().flush().expect("Failed to flush stdout!");
    input.to_string()
}

async fn connect(usr_name: String, addr: String) -> std::io::Result<()> {
    println!("Client '{usr_name}' connecting to {addr}");
    let mut stream = TcpStream::connect(addr)?;
    stream.write(usr_name.as_bytes())?;
    let mut line = [0; 256];
    stream.read(&mut line)?;
    chat(stream, str::from_utf8(&line).unwrap()).await;
    Ok(())
}

async fn listen(usr_name: String, port: usize) -> std::io::Result<()> {
    let addr = local_ip().unwrap_or(IpAddr::from_str("127.0.0.1").unwrap());
    let connection_addr = format!("{addr}:{port}");
    println!("Server '{usr_name}' listening to {connection_addr}");
    let listener = TcpListener::bind(connection_addr).await?;

    let mut stream = listener.accept().await?.0;
    stream.write(usr_name.as_bytes()).await?;
    let mut line = [0; 256];
    stream.read(&mut line).await?;
    chat(stream.into_std()?, str::from_utf8(&line).unwrap()).await;

    Ok(())
}

fn get_usr_name() -> String {
    print!("Enter your name: ");
    stdout().flush().expect("Unable to flush stdout");
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
            if let Err(err) = listen(
                get_usr_name(),
                args.last()
                    .unwrap_or(&"8888".to_string())
                    .parse()
                    .unwrap_or(8888),
            )
            .await
            {
                println!("An error occured in listen(): {err}");
            }
        } else if args[1] == "client" {
            if let Err(err) = connect(
                get_usr_name(),
                args.last()
                    .unwrap_or(&"127.0.0.1:8888".to_string())
                    .to_string(),
            )
            .await
            {
                println!("{err}: There is no server listening for connections.");
            } else {
                println!("Connection ended.");
            };
        }
    } else {
        println!("Enter 'server' or 'client' as an argument! (optional: also add port as argument, else 127.0.0.1:8888)");
    }
    Ok(())
}

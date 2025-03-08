use std::error::Error;
use std::io::{prelude::*, stdin, stdout, BufReader};
use std::net::{IpAddr, TcpStream};
use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::{env, str};
use std::{thread, time};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use local_ip_address::local_ip;

use crossterm::event::{read, KeyCode};
use crossterm::{cursor, event, queue};

const SLEEP_LENGTH: time::Duration = time::Duration::from_millis(100);
const WIDTH: usize = 74;
const HEIGHT: usize = 30;

async fn stream_io_thread(
    mut stream: TcpStream,
    other_usr: String,
    line_channel: (Sender<u16>, Receiver<u16>),
) -> Sender<String> {
    let mut reader = BufReader::new(stream.try_clone().expect("Failed to clone stream!"));
    let (input_sender, input_receiver) = channel::<String>();
    let mut new_line_index: u16 = 1;
    let mut stdout = stdout();
    thread::spawn(move || loop {
        let mut line = String::new();
        if let Ok(line_counter) = line_channel.1.try_recv() {
            new_line_index = line_counter;
        }

        if let Ok(_err) = reader.read_line(&mut line) {
            if !line.is_empty() {
                stdout.flush().unwrap();
                let message = format!(
                    "\x1b[93m{}\x1b[0m: {}",
                    other_usr.trim_matches('\0'),
                    line.trim()
                );
                queue!(
                    stdout,
                    cursor::SavePosition,
                    cursor::MoveToRow(new_line_index)
                )
                .unwrap();
                new_line_index += 1 + (message.len() / WIDTH) as u16;
                if new_line_index > HEIGHT as u16 {
                    new_line_index = HEIGHT as u16;
                    let space_buffer: &str = &" ".repeat(WIDTH - 4 - message.len());
                    let message = format!("{message}{space_buffer}");
                    print!("\r┃ {}", message);
                    queue!(
                        stdout,
                        crossterm::terminal::ScrollUp(1),
                        cursor::MoveToPreviousLine(60)
                    )
                    .unwrap();
                    draw_gui();
                    queue!(stdout, cursor::MoveToRow(new_line_index)).unwrap();
                    print!("\r┃{}┃", " ".repeat(WIDTH - 2));
                    stdout.flush().unwrap();
                } else {
                    line_channel.0.send(new_line_index).unwrap();
                    print!("\r┃ {}", message);
                }
                queue!(stdout, cursor::RestorePosition).unwrap();
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
    let line_channel1 = channel::<u16>();
    let line_channel2 = channel::<u16>();
    let input_sender =
        stream_io_thread(stream, usr.to_string(), (line_channel1.0, line_channel2.1)).await;
    let mut stdout = stdout();
    queue!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        cursor::MoveToRow(0),
        cursor::MoveToColumn(0),
    )
    .unwrap();

    draw_gui();

    queue!(stdout, cursor::MoveToPreviousLine(3), cursor::MoveRight(2)).unwrap();
    let mut usr_input = String::new();
    crossterm::terminal::enable_raw_mode().unwrap();
    let mut new_line_index: u16 = 1;
    loop {
        if let Ok(line_counter) = line_channel1.1.try_recv() {
            new_line_index = line_counter;
        }

        if event::poll(SLEEP_LENGTH).unwrap() {
            let key_event = read().unwrap();

            if let event::Event::Key(k_event) = key_event {
                if let Ok(key_char) = char::from_str(&k_event.code.to_string()) {
                    if usr_input.len() < 128 {
                        print!("{}", k_event.code);
                        usr_input.push(key_char);
                    }
                    let col = cursor::position().unwrap().0;
                    queue!(stdout, cursor::MoveToColumn(2),).unwrap();
                    print!(
                        "{}",
                        &usr_input[(usr_input.len() as i16 - (WIDTH - 4) as i16)
                            .clamp(0, WIDTH as i16) as usize..]
                    );
                    queue!(stdout, cursor::MoveToColumn(col)).unwrap();
                }
                if k_event.code == KeyCode::Char(' ') {
                    print!(" ");
                    usr_input.push(' ');
                }
                if k_event.code == KeyCode::Backspace && !usr_input.is_empty() {
                    queue!(stdout, cursor::MoveLeft(1)).unwrap();
                    print!(" ");
                    queue!(stdout, cursor::MoveLeft(1)).unwrap();
                    usr_input.pop();
                }
                stdout.flush().unwrap()
            }

            //println!("Event::{:?}\r", key_event);

            if key_event == event::Event::Key(KeyCode::Enter.into()) && !usr_input.is_empty() {
                usr_input.push('\n');
                match input_sender.send(usr_input.clone()) {
                    Err(_e) => return,
                    Ok(f) => f,
                };
                queue!(stdout, cursor::MoveToRow(new_line_index)).unwrap();
                new_line_index += 1 + (usr_input.len() / WIDTH) as u16;
                if new_line_index > HEIGHT as u16 {
                    new_line_index = HEIGHT as u16;
                    let space_buffer: &str = &" ".repeat(WIDTH - 4 - usr_input.len());
                    print!("\r┃ > {}{space_buffer}┃", usr_input);
                    queue!(
                        stdout,
                        crossterm::terminal::ScrollUp(1),
                        cursor::MoveToPreviousLine(50)
                    )
                    .unwrap();
                    draw_gui();
                    queue!(stdout, cursor::MoveToRow(new_line_index)).unwrap();
                    print!("\r┃{}┃", " ".repeat(WIDTH - 2));
                    stdout.flush().unwrap();
                } else {
                    line_channel2.0.send(new_line_index).unwrap();
                    print!("\r┃ > {}", usr_input);
                }
                usr_input.clear();
                queue!(stdout, cursor::MoveToRow(HEIGHT as u16 + 4)).unwrap();
                print!("\r┃{}┃\r", " ".repeat(WIDTH - 2));
                queue!(stdout, cursor::MoveRight(2)).unwrap();
                stdout.flush().unwrap();
            }

            if key_event == event::Event::Key(KeyCode::Esc.into()) {
                return;
            }
        }
    }
}

fn draw_gui() {
    let mut stdout = stdout();
    println!("\r┏━━LAN-Chat{}┓", "━".repeat(WIDTH - 12));
    for _ in 0..HEIGHT {
        queue!(
            stdout,
            crossterm::style::Print("\r┃"),
            cursor::MoveToColumn(WIDTH as u16 - 1),
            crossterm::style::Print("┃\n")
        )
        .unwrap();
    }
    println!("\r┗{}┛", "━".repeat(WIDTH - 2));
    println!("\r┏━━Enter Message:{}┓", "━".repeat(WIDTH - 18));
    for _ in 0..3 {
        println!("\r┃{}┃", " ".repeat(WIDTH - 2));
    }
    println!("\r┗{}┛", "━".repeat(WIDTH - 2)); // WIDTH
}

async fn connect(usr_name: String, addr: String) -> std::io::Result<()> {
    println!("Client '{usr_name}' connecting to {addr}");
    let mut stream = TcpStream::connect(addr)?;
    stream.write(usr_name.as_bytes())?;
    let mut line = [0; 128];
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
    let mut line = [0; 128];
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

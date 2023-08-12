use tokio::{
    io::{BufReader, stdin, AsyncReadExt}, 
    sync::mpsc::UnboundedSender
};
use tokio_serial::SerialPortBuilderExt;
use tokio::io::AsyncWriteExt;
use std::io::{Write, stdout};
use std::process::exit;
use clap::Parser;
use crossterm::terminal;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    path: String,

    #[arg(short, long)]
    baud_rate: u32,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    monitor(cli.path, cli.baud_rate).await;
    shutdown();
}

async fn monitor(path: String, baud_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    let port = tokio_serial::new(&path, baud_rate).open_native_async().expect("Cannot open port");
    println!("Connected to {}\n\
        Press Ctrl + ] to exit", path);
    let (mut reader, mut writer) = tokio::io::split(port);
    let mut stdout = stdout();
    terminal::enable_raw_mode().expect("Failed enabling raw mode");

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async {
        read_user_input(sender).await;
    });

    loop {
        let mut buf: Vec<u8> = Vec::new();
        tokio::select! {
            len = reader.read_buf(&mut buf) => match len {
                Ok(_) => {
                    let input = String::from_utf8_lossy(&buf);
                    write!(stdout, "{input}").unwrap();
                    stdout.flush()?;
                },
                Err(e) => {
                    println!("{e}");
                    break Err(Box::new(e));
                }
            },
            Some(bytes) = receiver.recv() => {
                for byte in &bytes {
                    match byte {
                        29 => return Ok(()),
                        _ => {}
                    }
                }
                writer.write_all(&bytes).await?;
            
            }
        }
    }
}

async fn read_user_input(sender: UnboundedSender<Vec<u8>>) {
    let mut reader = BufReader::new(stdin());
    let mut buf: [u8; 1024] = [0; 1024];

    loop {
        let bytes = match reader.read(&mut buf[..]).await {
            Ok(bytes) if bytes > 0 => bytes,
            Ok(_) => break, // End of input
            Err(e) => {
                eprintln!("Error reading from stdin: {:?}", e);
                break;
            }
        };

        sender.send(buf[..bytes].to_vec()).expect("Failed to send data through channel"); 
    }
}

fn shutdown() {
    crossterm::terminal::disable_raw_mode().expect("Failed disabling raw mode");
    println!();
    
    exit(0);
}
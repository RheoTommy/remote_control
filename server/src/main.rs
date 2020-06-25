#![windows_subsystem = "windows"]

extern crate bincode;
extern crate common;

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;

fn main() {
    let listener = TcpListener::bind("0.0.0.0:1234").expect("OMG! couldn't bind!");
    for streams in listener.incoming() {
        match streams {
            Err(e) => eprintln!("{:?}", e),
            Ok(stream) => {
                std::thread::spawn(move || {
                    let res = handler(stream);
                    if let Err(e) = res {
                        eprintln!("{:?}", e.msg);
                    }
                });
            }
        }
    }
}

struct MyError {
    msg: String,
}

impl From<std::io::Error> for MyError {
    fn from(e: std::io::Error) -> Self {
        MyError { msg: e.to_string() }
    }
}

impl From<bincode::Error> for MyError {
    fn from(e: bincode::Error) -> Self {
        MyError { msg: e.to_string() }
    }
}

fn handler(mut stream: TcpStream) -> Result<(), MyError> {
    use common::remote_control::*;
    let mut buf = [0; 1024];
    stream.read(&mut buf)?;
    let msg: MessageType = bincode::deserialize(&buf[..])?;

    let msg = match msg {
        MessageType::SimpleMessage(s) => format!("Echo : {}", s),
        MessageType::RunCommand(cmd) => {
            let output = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .arg("/C")
                    .arg(OsStr::new(&cmd))
                    .output()?
            } else {
                Command::new("sh").arg("-c").arg("echo hello").output()?
            };

            format!(
                "stdout:\n{}\nstderr\n{}",
                String::from_utf8(output.stdout)
                    .unwrap_or("could'nt convert stdout to string!".to_string()),
                String::from_utf8(output.stderr)
                    .unwrap_or("couldn't convert stderr to string!".to_string())
            )
        }
        MessageType::End => "Good Bye".to_string(),
    };

    let msg = bincode::serialize(&msg)?;
    stream.write_all(&msg)?;
    stream.flush()?;
    Ok(())
}

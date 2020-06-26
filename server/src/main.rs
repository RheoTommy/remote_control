#![windows_subsystem = "windows"]

extern crate bincode;
extern crate common;
extern crate encoding_rs;

use common::remote_control::*;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::thread::JoinHandle;

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

fn handler(mut stream: TcpStream) -> Result<(), MyError> {
    let mut buf = [0; 1024];
    stream.read(&mut buf)?;
    let msg: MessageType = bincode::deserialize(&buf[..])?;
    
    let msg = process_msg(msg);
    let msg = match msg {
        Err(e) => format!("Error while processing : {}", e.msg),
        Ok(e) => e,
    };
    
    stream.write_all((&msg[..]).as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn process_msg(msg: MessageType) -> Result<String, MyError> {
    let msg = match msg {
        MessageType::Echo(s) => format!("Echo : {}", s),
        MessageType::RunCommand {
            command: cmd,
            is_waiting,
        } => {
            let mut command = if cfg!(target_os = "windows") {
                let mut c = Command::new("cmd");
                c.arg("/C").arg(OsStr::new(&cmd));
                c
            } else {
                let mut c = Command::new("sh");
                c.arg("-c").arg("echo hello");
                c
            };
            
            if is_waiting {
                let (sender, receiver) = std::sync::mpsc::channel();
                let _thread: JoinHandle<Result<(), MyError>> = std::thread::spawn(move || {
                    let output = command.output()?;
                    let s = format!(
                        "stdout :\n{}\nstderr:\n{}\n",
                        encoding_rs::SHIFT_JIS
                            .decode(&output.stdout)
                            .0
                            .to_string()
                            .trim()
                            .trim_end(),
                        encoding_rs::SHIFT_JIS
                            .decode(&output.stderr)
                            .0
                            .to_string()
                            .trim()
                            .trim_end()
                    );
                    sender.send(s)?;
                    Ok(())
                });
                receiver.recv_timeout(std::time::Duration::new(3, 0))?
            } else {
                "Ran the command but I don't know if it's ok".to_string()
            }
        }
        MessageType::End => "Good Bye".to_string(),
        MessageType::SendFile { filename, contents } => {
            let mut f = File::create(&filename)?;
            f.write_all(&contents[..].as_bytes())?;
            f.flush()?;
            "Created the file".to_string()
        }
    };
    
    Ok(msg)
}

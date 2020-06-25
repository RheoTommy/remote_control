extern crate bincode;
extern crate common;
extern crate encoding_rs;
extern crate serde;

use common::remote_control::*;
use std::convert::*;
use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Debug, Clone, Eq, PartialEq)]
struct MyError {
    msg: String,
}

impl From<std::io::Error> for MyError {
    fn from(e: std::io::Error) -> Self {
        MyError { msg: e.to_string() }
    }
}

impl From<&str> for MyError {
    fn from(s: &str) -> Self {
        MyError { msg: s.to_string() }
    }
}

impl From<bincode::Error> for MyError {
    fn from(e: bincode::Error) -> Self {
        MyError { msg: e.to_string() }
    }
}

fn parse_line() -> Result<MessageType, MyError> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let mut input = buf.split_whitespace();
    
    match input.next().ok_or("no arguments!")? {
        ty if &ty.to_lowercase() == "simplemessage" || ty == "SM" => {
            let mut input = input.peekable();
            let is_none = input.peek().is_none();
            if is_none {
                Err(MyError {
                    msg: "not enough arguments! SimpleMessage need a message!".to_string(),
                })
            } else {
                Ok(MessageType::SimpleMessage(
                    input.collect::<Vec<&str>>().join(" "),
                ))
            }
        }
        
        ty if &ty.to_lowercase() == "runcommand" || ty == "RC" => {
            let mut input = input.peekable();
            let is_none = input.peek().is_none();
            if is_none {
                Err(MyError {
                    msg: "not enough arguments! RunCommand need a command!".to_string(),
                })
            } else {
                Ok(MessageType::RunCommand(
                    input.collect::<Vec<&str>>().join(" "),
                ))
            }
        }
        ty if &ty.to_lowercase() == "end" => {
            if input.next().is_some() {
                Err(MyError {
                    msg: "unexpected arguments! arguments are too many!".to_string(),
                })
            } else {
                Ok(MessageType::End)
            }
        }
        _ => Err(MyError {
            msg: "unexpected input type!".to_string(),
        }),
    }
}

fn main() {
    println!("{}", "Type IP Address and port");
    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .expect("cannot read the line on the standard input stream!");
    let mut iter = buf.split_whitespace();
    let ip = iter.next().expect("not enough arguments! need ip address!");
    let port = iter.next().expect("not enough arguments! need port!");
    let ip_address = format!("{}:{}", ip, port);
    let ip_address = &ip_address;
    
    loop {
        let mut stream = TcpStream::connect(ip_address).expect("cannot connect to port");
        println!("type command.");
        let res = parse_line();
        
        match res {
            Err(e) => {
                eprintln!("{:?}", e.msg);
                eprintln!("please type correct command!\n");
            }
            Ok(mt) => match mt {
                MessageType::End => {
                    match send(&mut stream, &mt) {
                        Err(e) => {
                            eprintln!("{:?}", e);
                        }
                        Ok(msg) => {
                            println!("Server : {}", msg);
                        }
                    };
                    println!("{}", "exiting the process");
                    std::process::exit(0);
                }
                _ => match send(&mut stream, &mt) {
                    Err(e) => {
                        eprintln!("{:?}", e);
                        continue;
                    }
                    Ok(msg) => {
                        println!("{}\n", msg);
                    }
                },
            },
        }
    }
}

fn send(stream: &mut TcpStream, data: &MessageType) -> Result<String, MyError> {
    let bytes = bincode::serialize(data).map_err(|e| MyError {
        msg: format!("While serializing data to bytes : {}", e.to_string()),
    })?;
    
    stream.write(&bytes).map_err(|e| MyError {
        msg: format!("While writing the bytes : {}", e.to_string()),
    })?;
    
    stream.flush().map_err(|e| MyError {
        msg: format!("While flushing the bytes : {}", e.to_string()),
    })?;
    
    let mut buf = [0; 2048];
    stream.read(&mut buf).map_err(|e| MyError {
        msg: format!("While reading the buf : {}", e.to_string()),
    })?;
    
    Ok(encoding_rs::SHIFT_JIS
        .decode(&buf)
        .0
        .to_string()
        .trim()
        .trim_end()
        .to_string())
}

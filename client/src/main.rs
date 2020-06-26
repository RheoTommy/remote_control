extern crate bincode;
extern crate common;
extern crate encoding_rs;
extern crate serde;

use common::remote_control::*;
use std::fs::{read_dir, File};
use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;

fn parse_line() -> Result<MessageType, MyError> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let mut input = buf.split_whitespace();
    let ty = input.next().ok_or("no arguments!")?;
    let ty_lower = &ty.to_lowercase();
    match ty {
        ty if ty_lower == "simplemessage" || ty == "SM" => {
            let mut input = input.peekable();
            if input.peek().is_none() {
                Err(MyError {
                    msg: "not enough arguments! SimpleMessage need a message!".to_string(),
                })
            } else {
                Ok(MessageType::Echo(input.collect::<Vec<&str>>().join(" ")))
            }
        }
        
        ty if ty_lower == "runcommand" || ty == "RC" => {
            let mut input = input.peekable();
            if input.peek().is_none() {
                Err(MyError {
                    msg: "not enough arguments! RunCommand need a command!".to_string(),
                })
            } else {
                let is_waiting = if &input.peek().unwrap().to_lowercase() == "-w" {
                    true
                } else {
                    false
                };
                Ok(MessageType::RunCommand {
                    command: input.skip(1).collect::<Vec<&str>>().join(" "),
                    is_waiting,
                })
            }
        }
        ty if ty_lower == "end" || &ty.to_lowercase() == "exit" => {
            if input.next().is_some() {
                Err(MyError {
                    msg: "unexpected arguments! arguments are too many!".to_string(),
                })
            } else {
                Ok(MessageType::End)
            }
        }
        ty if ty_lower == "sendfile" || ty == "SF" => {
            let mut input = input.peekable();
            if input.peek().is_none() {
                Err(MyError {
                    msg: "not enough arguments! SendFile need a filepath".to_string(),
                })
            } else {
                let filename = input.next().unwrap().to_string();
                let new_filename = input.next().unwrap_or(&filename);
                let contents = send_file(&filename)?;
                Ok(MessageType::SendFile {
                    filename: new_filename.to_string(),
                    contents,
                })
            }
        }
        ty if ty == "ls" => {
            let f = read_dir(".\\")?;
            let mut s = String::new();
            for path in f {
                s.push_str(&format!("Name : {}\n", path.unwrap().path().display()))
            }
            Ok(MessageType::Echo(s))
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
    let ip_address = &format!("{}:{}", ip, port);
    
    loop {
        println!("type command.");
        let res = parse_line();
        let mut stream = TcpStream::connect(ip_address).expect("cannot connect to port");
        
        
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
                            println!("Server : {}", msg.trim().trim_end());
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
                        println!("{}\n", msg.trim().trim_end());
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
    
    let mut buf = String::new();
    stream.read_to_string(&mut buf).map_err(|e| MyError {
        msg: format!("While reading the buf : {}", e.to_string()),
    })?;
    
    Ok(buf)
    
    // Ok(encoding_rs::SHIFT_JIS
    //     .decode(&buf)
    //     .0
    //     .to_string()
    //     .trim()
    //     .trim_end()
    //     .to_string())
}

fn send_file(s: &str) -> Result<String, MyError> {
    let mut f = File::open(s)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok(buf)
}

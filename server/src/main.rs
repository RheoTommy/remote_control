extern crate bincode;
extern crate common;

use common::remote_control::*;
use std::fs::{read_dir, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::{fs, io};
use ws::{listen, Handler, Message, Sender};
use ws::{CloseCode, Result as WResult};

fn main() {
    let configfile_path: &Path = Path::new("ip.config");
    if !configfile_path.exists() {
        let res = make_ipconfig();
        if let Err(me) = res {
            log_error(me);
            std::process::exit(-1);
        }
    }
    let config = MyConfig::from_configfile(configfile_path).unwrap_or_else(|e| {
        log_error(e);
        std::process::exit(-1);
    });
    
    let ip = format!("{}:{}", config.ip, config.port);
    
    listen(ip, |out| Server { out }).unwrap();
}

struct Server {
    out: Sender,
}

impl Handler for Server {
    fn on_message(&mut self, msg: Message) -> WResult<()> {
        match msg {
            Message::Text(txt) => {
                println!("{}", txt);
            }
            Message::Binary(bytes) => {
                let msg = bincode::deserialize(&bytes).unwrap_or(MyResponse::Ok(
                    MyResponseKind::Echo("受け取ったResponseKindの解凍に失敗しました".to_string()),
                ));
                process_response(msg);
            }
        }
        
        match process() {
            None => {
                self.out.close(CloseCode::Normal).expect("接続を切断する際にエラーが発生しました");
                std::process::exit(0);
            }
            Some(mm) => {
                let mm = bincode::serialize(&mm).unwrap_or(Default::default());
                self.out.send(Message::Binary(mm))
            }
        }
    }
}

fn process() -> Option<MyMessage> {
    println!("コマンドを入力してください");
    match parse_line() {
        Err(e) => {
            eprintln!("{}", e);
            process()
        }
        Ok(pk) => match pk {
            ParseKind::End => None,
            ParseKind::Ls => process(),
            ParseKind::Help => process(),
            ParseKind::Echo(s) => Some(MyMessage::Echo(s)),
            ParseKind::RunCommand {
                command,
                is_waiting,
            } => Some(MyMessage::RunCommand {
                command,
                is_waiting,
            }),
            ParseKind::SendFile { filename, contents } => {
                Some(MyMessage::SendFile { filename, contents })
            }
        },
    }
}

fn process_response(res: MyResponse) {
    match res {
        Ok(mrk) => match mrk {
            MyResponseKind::Echo(s) => {
                println!("{}", s);
            }
            MyResponseKind::RunCommand { stdout, stderr } => {
                println!("stdout :/n{}", stdout);
                eprintln!("stderr :\n{}", stderr);
            }
            MyResponseKind::SendFile => {
                println!("ファイルを送信しました");
            }
        },
        Err(me) => {
            eprintln!("{}\n", me);
        }
    }
}

fn parse_line() -> Result<ParseKind, MyError> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).map_err(|e| {
        MyError::new(
            e,
            "標準入力から一行を受け取る際にエラーが発生しました".to_string(),
        )
    })?;
    let mut input = buf.split_whitespace();
    let ty = input.next().ok_or("コマンドを入力してください").map_err(|e| {
        MyError::new(
            e,
            "コマンドを受け取って解析する際にエラーが発生しました".to_string(),
        )
    })?;
    
    let ty_lower = &ty.to_lowercase();
    match ty {
        ty if ty_lower == "simplemessage" || ty == "SM" => {
            let mut input = input.peekable();
            if input.peek().is_none() {
                Err(MyError {
                    msg: "メッセージに当たる引数がありません".to_string(),
                    when: "SimpleMessageの引数を確認している際にエラーが発生しました".to_string(),
                })
            } else {
                Ok(ParseKind::Echo(input.collect::<Vec<&str>>().join(" ")))
            }
        }
        
        ty if ty_lower == "runcommand" || ty == "RC" => {
            let mut input = input.peekable();
            if input.peek().is_none() {
                Err(MyError {
                    msg: "コマンドに当たる引数がありません".to_string(),
                    when: "RunCommandの引数を確認している際にエラーが発生しました".to_string(),
                })
            } else {
                let is_waiting = if &input.peek().unwrap().to_lowercase() == "-w" {
                    true
                } else {
                    false
                };
                Ok(ParseKind::RunCommand {
                    command: input.skip(1).collect::<Vec<&str>>().join(" "),
                    is_waiting,
                })
            }
        }
        ty if ty_lower == "end" || &ty.to_lowercase() == "exit" => {
            if input.next().is_some() {
                Err(MyError {
                    msg: "引数が多すぎます".to_string(),
                    when: "Endの引数を確認している際にエラーが発生しました".to_string(),
                })
            } else {
                Ok(ParseKind::End)
            }
        }
        ty if ty_lower == "sendfile" || ty == "SF" => {
            let mut input = input.peekable();
            if input.peek().is_none() {
                Err(MyError {
                    msg: "ファイルパスに当たる引数がありません".to_string(),
                    when: "SendFileの引数を確認している際にエラーが発生しました".to_string(),
                })
            } else {
                let filename = input.next().unwrap().to_string();
                let new_filename = input.next().unwrap_or(&filename);
                let contents = send_file(&filename)?;
                Ok(ParseKind::SendFile {
                    filename: new_filename.to_string(),
                    contents,
                })
            }
        }
        ty if ty == "ls" => {
            let f = read_dir(".\\").map_err(|e| MyError::new(
                e,
                "プログラムが実行されているディレクトリのファイル一覧を獲得する際にエラーが発生しました".to_string(),
            ))?;
            let par = fs::canonicalize(&Path::new(".\\")).map_err(|e| {
                MyError::new(e, "絶対パスの取得の際にエラーが発生しました".to_string())
            })?;
            let mut s = String::new();
            s.push_str(&format!(
                "{}\n",
                par.to_str().unwrap_or("絶対パスを取得できませんでした")
            ));
            for path in f {
                s.push_str(&format!("{}\n", path.unwrap().path().display()))
            }
            println!("{}", s);
            Ok(ParseKind::Ls)
        }
        _ty if ty_lower == "help" => {
            let s = "\
help                実行できるコマンドを確認できます
end(exit)           プログラムを終了します
ls                  このプログラムの動いている絶対ディレクトリとそのディレクトリのファイル一覧を表示します
SendFile(SF)        ファイルを送信します
SimpleMessage(SM)   メッセージを送信します
RunCommand(RC)      コマンドを実行します
            ";
            println!("{}", s);
            Ok(ParseKind::Help)
        }
        _ => Err(MyError {
            msg: "間違ったコマンドです".to_string(),
            when: "コマンドを解析している際にエラーが発生しました".to_string(),
        }),
    }
}

fn send_file(s: &str) -> Result<String, MyError> {
    let path = Path::new(s);
    let mut f = File::open(path).map_err(|e| MyError::new(e, "送るファイルを開く際にエラーが発生しました".to_string()))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf).map_err(|e| {
        MyError::new(
            e,
            "送るファイルを読み込む際にエラーが発生しました".to_string(),
        )
    })?;
    Ok(buf)
}

fn make_ipconfig() -> Result<(), MyError> {
    let ip = get_ip()?;
    let ip = format!("{} {}", ip, "1234");
    let mut f = File::create("ip.config").map_err(|e| MyError::new(
        e,
        "ip.configを作成する際にエラーが発生しました".to_string(),
    ))?;
    f.write_all(ip.as_bytes()).map_err(|e| MyError::new(
        e,
        "ip.configに書き込む際にエラーが発生しました".to_string(),
    ))?;
    f.flush().map_err(|e| MyError::new(
        e,
        "ip.configに書き込む際にエラーが発生しました".to_string(),
    ))?;
    Ok(())
}

fn get_ip() -> Result<String, MyError> {
    let mut command = Command::new("cmd");
    let output = command.arg("/C").arg("ipconfig").output().map_err(|e| MyError::new(e, "Ipconfigの実行中にエラーが発生しました".to_string()))?;
    let output = encoding_rs::SHIFT_JIS.decode(&output.stdout).0.trim().trim_end().to_string();
    let ip = grep(&output, "IPv4");
    let ip = ip[1].trim().trim_end().split_whitespace();
    Ok(ip.last().ok_or_else(|| MyError::new(
        "Ipアドレスに当たる文字列がありません".to_string(),
        "Ipアドレスを抽出する際にエラーが発生しました".to_string(),
    ))?.to_string())
}

fn grep<'a, 'b>(contents: &'a str, s: &'b str) -> Vec<&'a str> {
    contents.lines().filter(|line| line.contains(s)).collect()
}

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
    let config = set_config();
    let ip = format!("{}:{}", config.ip, config.port);

    listen(ip, |out| Server {
        out,
        exec_number: 1,
    })
    .unwrap();
}

/// 必要に応じてファイルを生成し、MyConfigを返します
///
/// # Panics
/// * `make_ip_ini()`の実行時にファイル生成などでエラーが発生した場合
/// * `MyConfig::from_configfile()`の実行時にファイルの読み込みと解析などでエラーが発生した場合
///
/// `err.log`にログを残し、Panicします
fn set_config() -> MyConfig {
    let configfile_path: &Path = Path::new(CONFIG_FILE);
    if !configfile_path.exists() {
        let res = make_ip_ini();
        if let Err(me) = res {
            log_error(me);
            std::process::exit(-1);
        }
    }
    let config = MyConfig::from_configfile(configfile_path).unwrap_or_else(|e| {
        log_error(e);
        std::process::exit(-1);
    });
    config
}

struct Server {
    out: Sender,
    exec_number: usize,
}

impl Handler for Server {
    /// クライアントから帰ってきたMessageを解析し、適切な処理をします
    ///
    /// # Panics
    /// * `ProcessType::End`が送られてきた際WebSocketの切断を正常に行えないとPanicします
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
            ProcessType::End => {
                self.out
                    .close(CloseCode::Normal)
                    .expect("接続を切断する際にエラーが発生しました");
                std::process::exit(0);
            }
            ProcessType::SetExecNumber(n) => {
                self.exec_number = n;
                self.out.send(Message::Text("セットしました".to_string()))
            }
            ProcessType::NormalMessage(mm) => {
                let m = if let MyMessage::RunCommand {
                    exec_number: _,
                    command,
                } = mm
                {
                    MyMessage::RunCommand {
                        exec_number: self.exec_number,
                        command,
                    }
                } else {
                    mm
                };
                let m = bincode::serialize(&m).unwrap_or(Default::default());
                self.out.send(Message::Binary(m))
            }
        }
    }
}

/// 次の処理をコマンドラインで要求し、適切なProcessTypeを返します。
fn process() -> ProcessType {
    println!("\nコマンドを入力してください");
    match parse_line() {
        Err(e) => {
            eprintln!("{}", e);
            process()
        }
        Ok(pk) => match pk {
            ParseKind::End => ProcessType::End,
            ParseKind::Ls => process(),
            ParseKind::Help => process(),
            ParseKind::Echo(s) => ProcessType::NormalMessage(MyMessage::Echo(s)),
            ParseKind::RunCommand { command } => {
                ProcessType::NormalMessage(MyMessage::RunCommand {
                    command,
                    exec_number: 1,
                })
            }
            ParseKind::SendFile { filename, contents } => {
                ProcessType::NormalMessage(MyMessage::SendFile { filename, contents })
            }
            ParseKind::SetExecNumber(n) => ProcessType::SetExecNumber(n),
        },
    }
}

/// クライアントから帰ってきたMyResponseに対して適切な処理をします
fn process_response(res: MyResponse) {
    match res {
        Ok(mrk) => match mrk {
            MyResponseKind::Echo(s) => {
                println!("{}", s);
            }
            MyResponseKind::RunCommand { stdout, stderr } => {
                println!("stdout :\n{}", stdout);
                eprintln!("stderr :\n{}", stderr);
            }
            MyResponseKind::SendFile => {
                println!("ファイルを送信しました");
            }
        },
        Err(me) => {
            eprintln!("{}", me);
        }
    }
}

/// 標準入力から一行を読み取り、ParseKindに変換して返します
///
/// # Errors
/// 各ParseKindにおいて、適切でない引数が与えられた際にMyErrorを返します
fn parse_line() -> Result<ParseKind, MyError> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).map_err(|e| {
        MyError::new(
            e,
            "標準入力から一行を受け取る際にエラーが発生しました".to_string(),
        )
    })?;
    let mut input = buf.split_whitespace();
    let ty = input
        .next()
        .ok_or("コマンドを入力してください")
        .map_err(|e| {
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
                Ok(ParseKind::RunCommand {
                    command: input.collect::<Vec<&str>>().join(" "),
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
                let contents = get_file_contents(&filename)?;
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
        _ if ty_lower == "help" => {
            let s = help_string();
            println!("{}", s);
            Ok(ParseKind::Help)
        }
        ty if ty_lower == "setexecnumber" || ty == "SN" => {
            let n = input
                .next()
                .ok_or_else(|| {
                    MyError::new(
                        "実行する回数に当たる引数が与えられていません".to_string(),
                        "SetExecNumberの引数を確認している際にエラーが発生しました".to_string(),
                    )
                })?
                .parse()
                .map_err(|e| {
                    MyError::new(
                        e,
                        "実行する回数に当たる引数が整数値で与えられていません".to_string(),
                    )
                })?;

            if input.next().is_some() {
                Err(MyError {
                    msg: "不要な引数が含まれています".to_string(),
                    when: "SetExecNumberの引数を確認している際にエラーが発生しました".to_string(),
                })
            } else {
                Ok(ParseKind::SetExecNumber(n))
            }
        }
        _ => Err(MyError {
            msg: "間違ったコマンドです".to_string(),
            when: "コマンドを解析している際にエラーが発生しました".to_string(),
        }),
    }
}

/// `Help`メッセージを返します
fn help_string() -> &str {
    "\
help                    実行できるコマンドを確認できます
end(exit)               プログラムを終了します
ls                      このプログラムの動いている絶対ディレクトリとそのディレクトリのファイル一覧を表示します
SendFile(SF)            ファイルを送信します
    第1引数に送信するファイルのパスを指定します
    第2引数に送信後のファイル名（拡張子込み）を指定します
        （オプションであり、デフォルトでは送信時のファイル名が使われます）
SimpleMessage(SM)       メッセージを送信します
    可変長引数として送信するメッセージを受け取ります。ただのエコーサーバーです
RunCommand(RC)          コマンドを実行します
    -w : コマンド実行を待機するというコマンドになります。実行結果を取得できます
    可変長引数として実行するコマンドを受け取ります
SetExecNumber(SN)       RunCommandの際のコマンドの実行回数を指定します
    第1引数に実行回数となる非負整数値を指定します
    2回以上を指定した際、RunCommandの-wオプションは無効となります"
}

/// 与えられた文字列からパスを作成し、指定されたファイルの中身を`String`で返します
///
/// # Errors
/// * ファイルを開くとき
/// * ファイルの中身を読み取るとき
///
/// に発生したエラーをMyErrorで返します
fn get_file_contents(s: &str) -> Result<String, MyError> {
    let path = Path::new(s);
    let mut f = File::open(path)
        .map_err(|e| MyError::new(e, "送るファイルを開く際にエラーが発生しました".to_string()))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf).map_err(|e| {
        MyError::new(
            e,
            "送るファイルを読み込む際にエラーが発生しました".to_string(),
        )
    })?;
    Ok(buf)
}

/// `ip.ini`ファイルが存在しなかった際、`get_ip()`を用いてIPアドレスを取得し、そのIPアドレスとPort番号1234を`ip.ini`に保存します。
///
/// # Errors
/// * `get_ip()`におけるエラー
/// * `ip.ini`作成時のエラー
/// * `ip.ini`書き込み時のエラー
/// * `ip.ini`フラッシュ時のエラー
///
/// をMyErrorで返します
fn make_ip_ini() -> Result<(), MyError> {
    let ip = get_ip()?;
    let ip = format!("{} {}", ip, "1234");
    let mut f = File::create(CONFIG_FILE)
        .map_err(|e| MyError::new(e, "ip.iniを作成する際にエラーが発生しました".to_string()))?;
    f.write_all(ip.as_bytes())
        .map_err(|e| MyError::new(e, "ip.iniに書き込む際にエラーが発生しました".to_string()))?;
    f.flush()
        .map_err(|e| MyError::new(e, "ip.iniに書き込む際にエラーが発生しました".to_string()))?;
    Ok(())
}

/// `ipconfig`コマンドを実行し、実行結果から目的のIPv4アドレスをStringで返します
///
/// 対象のパソコンはWindowsのみを想定しています。
///
/// # Errors
/// * コマンド実行時
/// * コマンド実行結果の読み込み
/// * IPv4アドレスが見つからないとき
///
/// にMyErrorを返します
fn get_ip() -> Result<String, MyError> {
    let mut command = Command::new("cmd");
    let output = command
        .arg("/C")
        .arg("ipconfig")
        .output()
        .map_err(|e| MyError::new(e, "Ipconfigの実行中にエラーが発生しました".to_string()))?;
    let output = encoding_rs::SHIFT_JIS
        .decode(&output.stdout)
        .0
        .trim()
        .trim_end()
        .to_string();
    let ip = grep(&output, "IPv4");
    let b = grep(&output, "WSL");
    let n = if b.is_empty() { 0 } else { 1 };
    let ip = ip
        .get(n)
        .ok_or_else(|| {
            MyError::new(
                "対応するIPv4アドレスが見つかりません".to_string(),
                "ipconfigの実行結果を解析する際にエラーが発生しました".to_string(),
            )
        })?
        .trim()
        .trim_end()
        .split_whitespace();
    Ok(ip
        .last()
        .ok_or_else(|| {
            MyError::new(
                "Ipアドレスに当たる文字列がありません".to_string(),
                "Ipアドレスを抽出する際にエラーが発生しました".to_string(),
            )
        })?
        .to_string())
}

/// `&str`を2つ引数`a`,`b`に取り、`a`の各行に対して`b`が含まれているかチェックし、含まれている行のみを`Vec<&str>`で返します
fn grep<'a, 'b>(contents: &'a str, s: &'b str) -> Vec<&'a str> {
    contents.lines().filter(|line| line.contains(s)).collect()
}

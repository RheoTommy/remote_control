pub mod remote_control {
    extern crate bincode;
    extern crate serde;
    extern crate serde_derive;

    use serde_derive::*;
    use std::fmt::Display;

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub enum MessageType {
        Echo(String),
        RunCommand { command: String, is_waiting: bool },
        SendFile { filename: String, contents: String },
        End,
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub struct MyError {
        pub msg: String,
    }

    impl<T: Display> From<T> for MyError {
        fn from(t: T) -> Self {
            MyError { msg: t.to_string() }
        }
    }
}

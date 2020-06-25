pub mod remote_control {
    extern crate serde;
    extern crate serde_derive;

    use serde_derive::*;

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub enum MessageType {
        SimpleMessage(String),
        RunCommand(String),
        End,
    }
}

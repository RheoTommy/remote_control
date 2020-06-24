pub mod remote_control {
    #[derive(Debug,Clone,Eq, PartialEq)]
    pub enum MessageType {
        SimpleMessage(String),
        RunCommand(String),
        End,
    }
}

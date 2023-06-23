

pub enum MessageType {
    Normal {
        destination_uid: u16,
        length: u8,
        data: [u8; 64],
    },
    Ping,
}

pub struct Message {
    sender_uid: u16,
    message_type: MessageType,
}

pub struct MessageBuilder {
    sender_uid: u16,
}

impl MessageBuilder {
    pub fn new(sender_uid: u16) -> Self {
        MessageBuilder {
            sender_uid,
        }
    }

    pub fn ping(&self) -> Message {
        Message {
            sender_uid: self.sender_uid,
            message_type: MessageType::Ping,
        }
    }
}

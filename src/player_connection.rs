use tokio::sync::Mutex;
use wtransport::Connection;

use crate::{recv_messages::AnyEventPacket, send_messages::SendEvent};

pub struct PlayerConnection {
    pub w_connection: Connection,
    pub send_bi_stream: Mutex<wtransport::SendStream>,
}

impl PlayerConnection {
    pub fn new(
        w_connection: Connection,
        send_bi_stream: Mutex<wtransport::SendStream>,
    ) -> PlayerConnection {
        PlayerConnection {
            w_connection,
            send_bi_stream,
        }
    }

    pub async fn emit_bi_buffer(&self, buffer: &Vec<u8>) {
        let mut send_stream = self.send_bi_stream.lock().await;
        let _ = send_stream.write(&buffer).await;
    }

    pub async fn emit_bi<T: serde::Serialize>(&self, send_event: SendEvent, data: T) {
        let packet = AnyEventPacket::new(
            send_event,
            data,
        );

        let buffer = packet.to_buffer();
        self.emit_bi_buffer(&buffer).await;
    }

    pub async fn emit_unidirection<T: serde::Serialize>(&self, send_event: SendEvent, data: T) {}
}

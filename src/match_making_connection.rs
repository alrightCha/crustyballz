use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};

use log::info;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub struct MatchMakingConnection {
    socket_sender: Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
}

impl MatchMakingConnection {
    pub async fn send_message(&self, message: Message) {
        self.socket_sender.lock().await.send(message).await.unwrap();
    }

    pub async fn setup_connection(url: String) -> MatchMakingConnection {
        let (ws_stream, _) = connect_async(&url)
            .await
            .expect("Failed to connect matchmaking service");
        let (write, read) = ws_stream.split();

        tokio::spawn(MatchMakingConnection::process_service_responses(read));

        MatchMakingConnection {
            socket_sender: Mutex::new(write),
        }
    }

    async fn process_service_responses(
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(msg) => info!("Received: {}", msg),
                Err(e) => info!("Error receiving message: {}", e),
            }
        }
    }
}

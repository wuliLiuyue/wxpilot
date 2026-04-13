use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::{oneshot, Mutex},
    time::timeout,
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use wxp_automator_protocol::{AutomatorRequest, AutomatorResponse};

type PendingResult = Result<serde_json::Value, AutomatorClientError>;
type PendingSender = oneshot::Sender<PendingResult>;
type WsWriter =
    futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomatorClientError {
    Connect(String),
    Serialize(String),
    Send(String),
    DuplicateRequestId(String),
    RequestTimedOut(String),
    Protocol(String),
    ConnectionClosed,
}

impl fmt::Display for AutomatorClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(message) => write!(f, "connect_error: {message}"),
            Self::Serialize(message) => write!(f, "serialize_error: {message}"),
            Self::Send(message) => write!(f, "send_error: {message}"),
            Self::DuplicateRequestId(id) => write!(f, "duplicate_request_id: {id}"),
            Self::RequestTimedOut(id) => write!(f, "request_timed_out: {id}"),
            Self::Protocol(message) => write!(f, "protocol_error: {message}"),
            Self::ConnectionClosed => write!(f, "connection_closed"),
        }
    }
}

impl std::error::Error for AutomatorClientError {}

#[derive(Clone)]
pub struct AutomatorConnection {
    writer: Arc<Mutex<WsWriter>>,
    pending: Arc<Mutex<HashMap<String, PendingSender>>>,
    request_timeout: Duration,
}

impl AutomatorConnection {
    const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

    pub async fn connect(url: &str) -> Result<Self, AutomatorClientError> {
        let (stream, _) = connect_async(url)
            .await
            .map_err(|error| AutomatorClientError::Connect(error.to_string()))?;
        let (writer, mut reader) = stream.split();
        let pending = Arc::new(Mutex::new(HashMap::<String, PendingSender>::new()));
        let pending_reader = pending.clone();

        tokio::spawn(async move {
            while let Some(message) = reader.next().await {
                let Ok(message) = message else { break };
                let Ok(text) = message.into_text() else {
                    continue;
                };
                let Ok(response) = serde_json::from_str::<AutomatorResponse>(&text) else {
                    continue;
                };
                let Some(id) = response.id else { continue };
                let sender = pending_reader.lock().await.remove(&id);
                let Some(sender) = sender else { continue };

                let outcome = match response.error {
                    Some(error) => Err(AutomatorClientError::Protocol(error.message)),
                    None => Ok(response.result.unwrap_or(serde_json::Value::Null)),
                };
                let _ = sender.send(outcome);
            }

            let remaining = {
                let mut pending = pending_reader.lock().await;
                pending
                    .drain()
                    .map(|(_, sender)| sender)
                    .collect::<Vec<_>>()
            };
            for sender in remaining {
                let _ = sender.send(Err(AutomatorClientError::ConnectionClosed));
            }
        });

        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            pending,
            request_timeout: Self::DEFAULT_REQUEST_TIMEOUT,
        })
    }

    pub fn with_request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = request_timeout;
        self
    }

    pub async fn send_with_id(
        &self,
        id: String,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, AutomatorClientError> {
        let request = AutomatorRequest {
            id: id.clone(),
            method: method.to_string(),
            params,
        };
        let payload = serde_json::to_string(&request)
            .map_err(|error| AutomatorClientError::Serialize(error.to_string()))?;
        let (tx, rx) = oneshot::channel();
        let mut pending = self.pending.lock().await;
        if pending.contains_key(&id) {
            return Err(AutomatorClientError::DuplicateRequestId(id));
        }
        pending.insert(id.clone(), tx);
        drop(pending);

        if let Err(error) = self
            .writer
            .lock()
            .await
            .send(Message::Text(payload.into()))
            .await
        {
            self.pending.lock().await.remove(&id);
            return Err(AutomatorClientError::Send(error.to_string()));
        }

        match timeout(self.request_timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(AutomatorClientError::ConnectionClosed),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(AutomatorClientError::RequestTimedOut(id))
            }
        }
    }
}

pub fn normalize_input_call(tag: &str) -> &'static str {
    match tag {
        "input" => "input.input",
        "textarea" => "textarea.input",
        _ => "unknown",
    }
}

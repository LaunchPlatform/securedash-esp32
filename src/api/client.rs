use core::time;
use esp_idf_svc::io::EspIOError;
use esp_idf_svc::ws::client::{
    EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent, WebSocketEventType,
};
use std::sync::mpsc::{channel, Sender};

#[derive(Debug, PartialEq)]
enum ExampleEvent {
    Connected,
    MessageReceived,
    Closed,
}

pub struct APIClient<'a> {
    ws_client: EspWebSocketClient<'a>,
}

impl<'a> APIClient<'a> {
    fn new(endpoint: String, timeout: time::Duration) -> anyhow::Result<Self> {
        let config = EspWebSocketClientConfig {
            // server_cert: Some(X509::pem_until_nul(SERVER_ROOT_CERT)),
            ..Default::default()
        };
        let (sender, receiver) = channel::<WebSocketEvent>();
        Ok(Self {
            ws_client: EspWebSocketClient::new(&endpoint, &config, timeout, move |event| {
                match event {
                    Ok(ws_event) => sender.send(*ws_event).unwrap(),
                    Err(err) => {}
                };
            })?,
        })
    }

    fn handle_event(&mut self, event: &Result<WebSocketEvent, EspIOError>) {
        if let Ok(event) = event {
            match event.event_type {
                WebSocketEventType::BeforeConnect => {
                    log::info!("Websocket before connect");
                }
                WebSocketEventType::Connected => {
                    log::info!("Websocket connected");
                }
                WebSocketEventType::Disconnected => {
                    log::info!("Websocket disconnected");
                }
                WebSocketEventType::Close(reason) => {
                    log::info!("Websocket close, reason: {reason:?}");
                }
                WebSocketEventType::Closed => {
                    log::info!("Websocket closed");
                }
                WebSocketEventType::Text(text) => {
                    log::info!("Websocket recv, text: {text}");
                }
                WebSocketEventType::Binary(binary) => {
                    log::info!("Websocket recv, binary: {binary:?}");
                }
                WebSocketEventType::Ping => {
                    log::info!("Websocket ping");
                }
                WebSocketEventType::Pong => {
                    log::info!("Websocket pong");
                }
            }
        }
    }
}

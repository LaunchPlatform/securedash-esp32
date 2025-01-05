use core::time;
use esp_idf_svc::io::EspIOError;
use esp_idf_svc::ws::client::{
    EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent, WebSocketEventType,
};
use std::cmp::PartialEq;
use std::ops::Deref;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Debug, PartialEq)]
enum APIError {
    AlreadyConnected,
}

#[derive(Debug)]
enum DesiredState {
    Connected,
    Disconnected,
}

#[derive(Debug, PartialEq)]
enum ConnectionState {
    Connecting,
    BeforeConnect,
    Connected,
    Disconnected,
}

struct APIState<'a> {
    desired_state: DesiredState,
    connection_state: ConnectionState,
    ws_config: Option<EspWebSocketClientConfig<'a>>,
    ws_client: Option<EspWebSocketClient<'a>>,
}

pub struct APIClient<'a> {
    endpoint: String,
    timeout: time::Duration,
    state: Arc<RwLock<APIState<'a>>>,
}


impl<'a> APIClient<'a> {
    pub fn new(endpoint: String, timeout: time::Duration) -> Self {
        let state = Arc::new(RwLock::new(APIState {
            desired_state: DesiredState::Disconnected,
            connection_state: ConnectionState::Disconnected,
            ws_config: None,
            ws_client: None,
        }));
        let weak_state = Arc::downgrade(&state);
        Self {
            endpoint,
            timeout,
            state,
        }
    }

    pub fn get_desired_state(&self) -> &DesiredState {
        &self.state.read().unwrap().desired_state
    }

    pub fn connect(&mut self) -> anyhow::<()> {
        let state = self.state.write().unwrap();
        if state.connection_state != ConnectionState::Disconnected {
            let conn_state = &state.connection_state;
            log::info!("Already in {conn_state} state, do nothing");
            return Err(APIError::AlreadyConnected);
        }
        state.desired_state = DesiredState::Connected;
        let config = EspWebSocketClientConfig {
            // server_cert: Some(X509::pem_until_nul(SERVER_ROOT_CERT)),
            ..Default::default()
        };
        let weak_state = Arc::downgrade(&self.state);
        state.ws_client = Some(EspWebSocketClient::new(&self.endpoint, &config, self.timeout, move |event| {
            let state = weak_state.upgrade();
            if let Some(state) = state {
                state.write().unwrap().handle_event(event);
            }
        })?);
        log::info!("Change desired state to Connected");
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.state.write().unwrap().desired_state = DesiredState::Disconnected;
        log::info!("Change desired state to Disconnected}")
    }
}

impl APIState {
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
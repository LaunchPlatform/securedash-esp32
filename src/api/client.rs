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
    EspIOError { error: EspIOError },
}

#[derive(Debug,Copy,Clone)]
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
    ws_client: Option<EspWebSocketClient<'a>>,
}

pub struct APIClient<'a> {
    endpoint: String,
    timeout: time::Duration,
    config: EspWebSocketClientConfig<'a>,
    state: Arc<RwLock<APIState<'a>>>,
}


impl<'a> APIClient<'a> {
    pub fn new(endpoint: String, timeout: time::Duration) -> Self {
        Self {
            endpoint,
            timeout,
            config: EspWebSocketClientConfig {
                // server_cert: Some(X509::pem_until_nul(SERVER_ROOT_CERT)),
                ..Default::default()
            },
            state: Arc::new(RwLock::new(APIState {
                desired_state: DesiredState::Disconnected,
                connection_state: ConnectionState::Disconnected,
                ws_client: None,
            })),
        }
    }

    pub fn get_desired_state(&self) -> DesiredState {
        self.state.read().unwrap().desired_state
    }

    pub fn connect(&mut self) -> Result<(), APIError> {
        let state = self.state.write().unwrap();
        if state.connection_state != ConnectionState::Disconnected {
            let conn_state = &state.connection_state;
            log::info!("Already in {conn_state:?} state, do nothing");
            return Err(APIError::AlreadyConnected);
        }
        state.desired_state = DesiredState::Connected;
        let weak_state = Arc::downgrade(&self.state);
        state.ws_client = Some(EspWebSocketClient::new(&self.endpoint, &self.config, self.timeout, move |event| {
                let state = weak_state.upgrade();
                if let Some(state) = state {
                    state.write().unwrap().handle_event(event);
                }
            }).map_err(|error| {APIError::EspIOError {error}})?);
        log::info!("Change desired state to Connected");
        Ok(())
    }

    pub fn disconnect(&mut self) {
        let mut write_lock = self.state.write();
        let state = write_lock.as_mut().unwrap();
        state.ws_client = None;
        state.desired_state = DesiredState::Disconnected;
        log::info!("Change desired state to Disconnected")
    }
}

impl<'a> APIState<'a> {
    fn handle_event(&mut self, event: &Result<WebSocketEvent, EspIOError>) {
        if let Ok(event) = event {
            match event.event_type {
                WebSocketEventType::BeforeConnect => {
                    log::info!("Websocket before connect");
                    self.connection_state = ConnectionState::BeforeConnect;
                }
                WebSocketEventType::Connected => {
                    log::info!("Websocket connected");
                    self.connection_state = ConnectionState::Connected;
                }
                WebSocketEventType::Disconnected => {
                    log::info!("Websocket disconnected");
                }
                WebSocketEventType::Close(reason) => {
                    log::info!("Websocket close, reason: {reason:?}");
                    self.connection_state = ConnectionState::Disconnected;
                }
                WebSocketEventType::Closed => {
                    log::info!("Websocket closed");
                    self.connection_state = ConnectionState::Disconnected;
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
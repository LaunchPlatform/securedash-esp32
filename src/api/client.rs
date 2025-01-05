use core::time;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embedded_svc::ws::FrameType;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::io::EspIOError;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::ws::client::{
    EspWebSocketClient, EspWebSocketClientConfig, WebSocketClosingReason, WebSocketEvent,
    WebSocketEventType,
};
use std::cmp::PartialEq;
use std::sync::{Arc, Mutex, RwLock};

const STATE_CHANNEL_QUEUE_SIZE: usize = 32;

#[derive(Debug, PartialEq)]
pub enum APIError {
    AlreadyConnected,
    NotConnectedYet,
    EspError { error: EspError },
    EspIOError { error: EspIOError },
}

#[derive(Debug, Copy, Clone)]
pub enum DesiredState {
    Connected,
    Disconnected,
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum ConnectionState {
    Connecting,
    BeforeConnect,
    Connected,
    Close {
        reason: Option<WebSocketClosingReason>,
    },
    Closed,
    Disconnected,
}
#[derive(Debug, PartialEq)]
pub enum APIEvent {
    StateChange {
        old_state: ConnectionState,
        new_state: ConnectionState,
    },
}

struct APIState {
    desired_state: DesiredState,
    connection_state: ConnectionState,
    channel: Arc<Channel<CriticalSectionRawMutex, APIEvent, STATE_CHANNEL_QUEUE_SIZE>>,
}

pub struct ChannelReceiver {
    channel: Arc<Channel<CriticalSectionRawMutex, APIEvent, STATE_CHANNEL_QUEUE_SIZE>>,
}

impl ChannelReceiver {
    pub fn unwrap(&self) -> Receiver<CriticalSectionRawMutex, APIEvent, STATE_CHANNEL_QUEUE_SIZE> {
        self.channel.receiver()
    }
}

pub struct APIClient<'a> {
    endpoint: String,
    timeout: time::Duration,
    config: EspWebSocketClientConfig<'a>,
    ws_client: Option<EspWebSocketClient<'a>>,
    state: Arc<RwLock<APIState>>,
}

impl<'a> APIClient<'a> {
    pub fn new(endpoint: &str, timeout: time::Duration) -> Self {
        let channel = Arc::new(Channel::<
            CriticalSectionRawMutex,
            APIEvent,
            STATE_CHANNEL_QUEUE_SIZE,
        >::new());
        Self {
            endpoint: endpoint.to_string(),
            timeout,
            config: EspWebSocketClientConfig {
                // server_cert: Some(X509::pem_until_nul(SERVER_ROOT_CERT)),
                ..Default::default()
            },
            ws_client: None,
            state: Arc::new(RwLock::new(APIState {
                desired_state: DesiredState::Disconnected,
                connection_state: ConnectionState::Disconnected,
                channel,
            })),
        }
    }

    pub fn get_desired_state(&self) -> DesiredState {
        self.state.read().unwrap().desired_state
    }

    pub fn get_connection_state(&self) -> ConnectionState {
        self.state.read().unwrap().connection_state
    }

    pub fn connect(&mut self) -> Result<(), APIError> {
        let mut write_lock = self.state.write();
        let state = write_lock.as_mut().unwrap();
        if state.connection_state != ConnectionState::Disconnected {
            let conn_state = &state.connection_state;
            log::info!("Already in {conn_state:?} state, do nothing");
            return Err(APIError::AlreadyConnected);
        }
        state.desired_state = DesiredState::Connected;
        let weak_state = Arc::downgrade(&self.state);
        self.ws_client = Some(
            EspWebSocketClient::new(&self.endpoint, &self.config, self.timeout, move |event| {
                let state = weak_state.upgrade();
                if let Some(state) = state {
                    state.write().unwrap().handle_event(event);
                }
            })
            .map_err(|error| APIError::EspIOError { error })?,
        );
        log::info!("Change desired state to Connected");
        Ok(())
    }

    pub fn disconnect(&mut self) {
        let mut write_lock = self.state.write();
        let state = write_lock.as_mut().unwrap();
        self.ws_client = None;
        state.desired_state = DesiredState::Disconnected;
        log::info!("Change desired state to Disconnected")
    }

    pub fn acquire_receiver(&mut self) -> ChannelReceiver {
        ChannelReceiver {
            channel: self.state.read().unwrap().channel.clone(),
        }
    }

    pub fn send(&mut self, frame_type: FrameType, frame_data: &[u8]) -> Result<(), APIError> {
        let mut write_lock = self.state.write();
        let state = write_lock.as_mut().unwrap();
        if state.connection_state != ConnectionState::Connected {
            return Err(APIError::NotConnectedYet);
        }
        let ws_client = self.ws_client.as_mut().unwrap();
        ws_client
            .send(frame_type, frame_data)
            .map_err(|error| APIError::EspError { error })
    }
}

impl APIState {
    fn set_state(&mut self, new_state: ConnectionState) {
        let old_state = self.connection_state;
        self.connection_state = new_state;
        let channel = self.channel.clone();
        block_on(async {
            channel
                .sender()
                .send(APIEvent::StateChange {
                    old_state,
                    new_state,
                })
                .await;
        });
    }

    fn handle_event(&mut self, event: &Result<WebSocketEvent, EspIOError>) {
        if let Ok(event) = event {
            match event.event_type {
                WebSocketEventType::BeforeConnect => {
                    log::info!("Websocket before connect");
                    self.set_state(ConnectionState::BeforeConnect);
                }
                WebSocketEventType::Connected => {
                    log::info!("Websocket connected");
                    self.set_state(ConnectionState::Connected);
                }
                WebSocketEventType::Disconnected => {
                    log::info!("Websocket disconnected");
                    self.set_state(ConnectionState::Disconnected);
                }
                WebSocketEventType::Close(reason) => {
                    log::info!("Websocket close, reason: {reason:?}");
                    self.set_state(ConnectionState::Close { reason });
                }
                WebSocketEventType::Closed => {
                    log::info!("Websocket closed");
                    self.set_state(ConnectionState::Closed);
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

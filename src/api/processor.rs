use crate::api::processor::Response::{Error, FetchFileChunk, GetInfo, ListFiles, Reboot};
use crate::api::websocket::{ConnectionState, SessionEvent, WebSocketSession};
use embedded_svc::ws::FrameType;
use esp_idf_svc::hal::gpio::Pull;
use esp_idf_svc::ping::Info;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};
use std::mem::MaybeUninit;
use std::time;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    GetInfo,
    ListFiles { path: String },
    FetchFile { path: String, chunk_size: u64 },
    Reboot,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    path: String,
    size: u64,
    #[serde(with = "serde_millis")]
    modified_at: time::Instant,
    #[serde(with = "serde_millis")]
    created_at: time::Instant,
    is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    version: String,
    wifi_ip: String,
    #[serde(with = "serde_millis")]
    local_time: time::Instant,
    disk_size: u64,
    disk_usage: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response<'a> {
    GetInfo {
        device_info: DeviceInfo,
    },
    ListFiles {
        path: String,
        files: Vec<File>,
    },
    FetchFileChunk {
        offset: u64,
        data: &'a [u8],
        is_final: bool,
    },
    Reboot,
    Error {
        message: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandRequest {
    pub id: String,
    pub command: Command,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse<'a> {
    pub id: String,
    pub response: Response<'a>,
}

pub struct Processor {
    pub info_producer: dyn Fn() -> anyhow::Result<DeviceInfo>,
}

impl Processor {
    fn get_info(&self) -> anyhow::Result<Response> {
        (self.info_producer)().map(|device_info: DeviceInfo| GetInfo { device_info })
    }

    fn list_file(&self, path: &str) -> anyhow::Result<Response> {
        Ok(ListFiles {
            path: path.to_string(),
            files: vec![],
        })
    }

    fn fetch_file(
        &self,
        req_id: &str,
        path: &str,
        chunk_size: u64,
        send: fn(CommandResponse),
    ) -> anyhow::Result<()> {
        let mut file = std::fs::File::open(path)?;
        let file_size = file.metadata()?.len();
        let mut buf = vec![0; chunk_size as usize];
        for offset in (0..file_size).step_by(chunk_size as usize) {
            file.read(&mut buf)?;
            assert_eq!(file.stream_position().unwrap(), offset);
            send(CommandResponse {
                id: req_id.to_string(),
                response: FetchFileChunk {
                    offset,
                    data: &buf,
                    is_final: offset + chunk_size >= file_size,
                },
            });
        }
        Ok(())
    }

    fn reboot(&self) -> anyhow::Result<Response> {
        // TODO: reboot
        Ok(Reboot {})
    }

    pub async fn process(
        &self,
        request: &CommandRequest,
        send: fn(CommandResponse),
    ) {
        let response: anyhow::Result<Response> = match &request.command {
            Command::GetInfo => self.get_info(),
            Command::ListFiles { path } => self.list_file(path),
            Command::FetchFile { path, chunk_size } => {
                match self.fetch_file(&*request.id, path, *chunk_size, send) {
                    Ok(_) => {
                        return;
                    }
                    Err(error) => Err(error),
                }
            }
            Command::Reboot => self.reboot(),
        };
        send(CommandResponse {
            id: request.id.clone(),
            response: response.unwrap_or_else(|error| Error {
                message: error.to_string(),
            }),
        });
    }
}

async fn read_events(mut client: WebSocketSession<'_>) {
    client.connect();
    loop {
        log::info!("Reading events ...");
        let event = client.acquire_receiver().unwrap().receive().await;
        log::info!("!!! RECEIVED {event:#?}");
        match event {
            SessionEvent::StateChange {
                new_state: ConnectionState::Connected,
                ..
            } => {
                client
                    .send(FrameType::Text(false), "hello there".as_bytes())
                    .unwrap();
            }
            _ => {}
        }
    }
}

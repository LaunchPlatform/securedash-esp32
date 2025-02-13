use crate::api::processor::Response::{Error, FetchFileChunk, GetInfo, ListFiles, Reboot};
use crate::api::websocket::{ConnectionState, SessionEvent, WebSocketSession};
use anyhow::anyhow;
use embedded_svc::ws::FrameType;
use serde::{Deserialize, Serialize};
use std::fs::read_dir;
use std::io::Read;
use std::path::Path;
use time::serde::timestamp::milliseconds;
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Command {
    GetInfo,
    ListFiles { path: String },
    FetchFile { path: String, chunk_size: u64 },
    Reboot,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct File {
    path: String,
    size: u64,
    #[serde(with = "milliseconds")]
    modified_at: OffsetDateTime,
    #[serde(with = "milliseconds")]
    created_at: OffsetDateTime,
    is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub version: String,
    pub wifi_ip: String,
    pub mount_path: String,
    #[serde(with = "milliseconds")]
    pub local_time: OffsetDateTime,
    pub total_volume_size: u64,
    pub free_volume_size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandRequest {
    pub id: String,
    pub command: Command,
}

#[derive(Debug, Serialize, Clone)]
pub struct CommandResponse<'a> {
    pub id: String,
    pub response: Response<'a>,
}

pub type DeviceInfoProducer = Box<dyn Fn() -> anyhow::Result<DeviceInfo>>;

pub struct Processor {
    pub device_info_producer: DeviceInfoProducer,
    pub root_dir: String,
}

impl Processor {
    fn get_info(&self) -> anyhow::Result<Response> {
        let result = (self.device_info_producer)();
        if let Ok(device_info) = &result {
            log::info!("Get device info {device_info:#?}");
        }
        result.map(|device_info: DeviceInfo| GetInfo { device_info })
    }

    fn list_files(&self, path: &str) -> anyhow::Result<Response> {
        let dir_path = Path::new(&self.root_dir).join(path);
        log::info!(
            "Listing files at {:?}",
            dir_path.to_str().unwrap_or("<Unknown>")
        );
        // Ideally we should find a way to learn the size of all files, but we need to
        // iterate over all files anyway... so.. maybe not? :/
        let mut files: Vec<File> = vec![];
        for entry in read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path().into_os_string().into_string().map_err(|e| {
                anyhow!(
                    "Failed to decode path with error: {}",
                    e.into_string().unwrap_or(String::from("Unknown"))
                )
            });
            if path.is_err() {
                continue;
            }
            let path = path.unwrap();
            let metadata = entry.metadata()?;
            files.push(File {
                path,
                size: metadata.len(),
                modified_at: metadata.modified()?.into(),
                // TODO: somehow there's a bug or what making create time always returns zero,
                //       may need to find a time to debug it
                created_at: metadata.created()?.into(),
                is_dir: metadata.is_dir(),
            })
        }
        Ok(ListFiles {
            path: path.to_string(),
            files,
        })
    }

    fn fetch_file<F>(
        &self,
        req_id: &str,
        path: &str,
        chunk_size: u64,
        mut send: F,
    ) -> anyhow::Result<()>
    where
        F: FnMut(CommandResponse),
    {
        log::info!("Fetch file at {:?}, chunk_size={}", path, chunk_size);
        let mut file = std::fs::File::open(path)?;
        let file_size = file.metadata()?.len();
        let mut buf = vec![0; chunk_size as usize];
        let mut count: usize = 0;
        let mut total_bytes: usize = 0;
        for offset in (0..file_size).step_by(chunk_size as usize) {
            let read_size = file.read(&mut buf)?;
            // TODO: somehow stream_position doesn't work correctly?
            // assert_eq!(file.stream_position().unwrap(), offset);
            send(CommandResponse {
                id: req_id.to_string(),
                response: FetchFileChunk {
                    offset,
                    data: &buf[..read_size],
                    is_final: offset + chunk_size >= file_size,
                },
            });
            count += 1;
            total_bytes += read_size;
        }
        log::info!(
            "Send fetch file response for {:?}, chunk_count={}, total_size={}",
            path,
            count,
            total_bytes
        );
        Ok(())
    }

    fn reboot(&self) -> anyhow::Result<Response> {
        // TODO: reboot
        Ok(Reboot {})
    }

    pub fn process<F>(&self, request: &CommandRequest, mut send: F)
    where
        F: FnMut(CommandResponse),
    {
        let response: anyhow::Result<Response> = match &request.command {
            Command::GetInfo => self.get_info(),
            Command::ListFiles { path } => self.list_files(path),
            Command::FetchFile { path, chunk_size } => {
                match self.fetch_file(&*request.id, path, *chunk_size, &mut send) {
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

pub async fn process_events(
    mut client: WebSocketSession<'_>,
    device_info_producer: DeviceInfoProducer,
    root_dir: String,
) {
    let mut processor: Option<Box<Processor>> = Some(Box::new(Processor {
        device_info_producer,
        root_dir,
    }));
    client.connect();

    loop {
        log::info!("Reading events ...");
        let event = client.acquire_receiver().unwrap().receive().await;
        match event {
            SessionEvent::StateChange {
                new_state: ConnectionState::Connected,
                ..
            } => {
                let last_processor = processor.unwrap();
                processor = Some(Box::new(Processor { ..*last_processor }));
            }
            SessionEvent::ReceiveText { text } => {
                let request: serde_json::Result<CommandRequest> = serde_json::from_str(&text);
                match request {
                    Ok(request) => {
                        log::info!("Processing request {:?}", request);
                        processor.as_mut().unwrap().process(
                            &request,
                            |response: CommandResponse| match response.response {
                                FetchFileChunk { .. } => client
                                    .send(
                                        FrameType::Binary(false),
                                        &rmp_serde::to_vec(&response).unwrap(),
                                    )
                                    .unwrap(),
                                _ => client
                                    .send(
                                        FrameType::Text(false),
                                        serde_json::to_string(&response).unwrap().as_bytes(),
                                    )
                                    .unwrap(),
                            },
                        )
                    }
                    Err(error) => {
                        log::error!("Failed to parse payload with error: {error}")
                    }
                }
            }
            _ => {}
        }
    }
}

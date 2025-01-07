mod api;
mod config;
mod storage;
mod usb;
mod wifi;

use crate::api::processor::{process_events, DeviceInfo, DeviceInfoProducer};
use crate::api::websocket::{ConnectionState, SessionEvent, WebSocketSession};
use crate::config::Config;
use crate::storage::spiflash::SPIFlashStorage;
use crate::usb::msc_device::{MSCDevice, MSCDeviceConfig};
use crate::wifi::session::WifiSession;
use embedded_svc::wifi::AuthMethod;
use embedded_svc::ws::FrameType;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::sntp::EspSntp;
use esp_idf_svc::sys::{esp, esp_vfs_fat_info, free};
use futures::executor::{LocalPool, LocalSpawner};
use futures::task::LocalSpawnExt;
use std::ffi::CString;
use std::path::Path;
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use time::OffsetDateTime;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

const CONFIG_PATH: Option<&str> = option_env!("CONFIG_PATH");
const DEFAULT_CONFIG_PATH: &str = "securedash.toml";
const PARTITION_LABEL: Option<&str> = option_env!("PARTITION_LABEL");
const DEFAULT_PARTITION_LABEL: &str = "storage";
const MOUNT_PATH: Option<&str> = option_env!("MOUNT_PATH");
const DEFAULT_MOUNT_PATH: &str = "/disk";

fn load_config(config_file: &str) -> Option<Config> {
    log::info!("Reading config from {}", config_file);
    let config = Config::read(&config_file);
    if let Err(error) = &config {
        log::error!("Failed to load config: {error}");
    }
    let config = config.ok();
    if let Some(config) = &config {
        log::info!("Loaded config: {config:#?}");
    }
    config
}

async fn run_async(spawner: LocalSpawner) -> Result<(), anyhow::Error> {
    let partition_label = PARTITION_LABEL.unwrap_or(DEFAULT_PARTITION_LABEL);
    let mount_path = MOUNT_PATH.unwrap_or(DEFAULT_MOUNT_PATH);
    let config_path = CONFIG_PATH.unwrap_or(DEFAULT_CONFIG_PATH);
    log::info!("Start {PKG_NAME} - version={VERSION}, partition_label={partition_label}, mount_path={mount_path}, config_path={config_path}");

    let mut storage = Box::new(SPIFlashStorage::new());
    storage.initialize_partition(partition_label)?;
    storage.mount(&mount_path, 5);

    let config = load_config(Path::new(mount_path).join(config_path).to_str().unwrap());

    let mut msc_device = MSCDevice::new(&MSCDeviceConfig { high_speed: true }, storage);
    msc_device.install()?;

    let peripherals = Peripherals::take()?;
    /*
    let mut wifi = WifiSession::new(SSID, PASSWORD, AuthMethod::WPA2Personal, peripherals.modem)?;
    wifi.connect().await?;
    log::info!("Connected wifi: {:#?}", wifi.get_ip_info());*/

    // Keep it around or else the SNTP service will stop
    let _sntp = EspSntp::new_default()?;
    log::info!("SNTP initialized");

    let mut button = PinDriver::input(peripherals.pins.gpio14)?;
    button.set_pull(Pull::Up)?;

    /*
    let mut client = WebSocketSession::new(API_ENDPOINT, Duration::from_secs(30));

    let captured_mount_path = mount_path.clone();
    let mount_path_c_str = CString::new(mount_path.as_bytes())?;
    let device_info_producer: DeviceInfoProducer = Box::new(move || {
        let mut total_volume_size: u64 = 0;
        let mut free_volume_size: u64 = 0;
        esp!(unsafe {
            esp_vfs_fat_info(
                mount_path_c_str.as_ptr(),
                &mut total_volume_size,
                &mut free_volume_size,
            )
        })?;
        Ok(DeviceInfo {
            version: VERSION.to_string(),
            // TODO: maybe pass in Rc of wifi instead?
            wifi_ip: wifi.get_ip_info().unwrap().ip.to_string(),
            local_time: OffsetDateTime::now_utc(),
            mount_path: captured_mount_path.to_string(),
            total_volume_size,
            free_volume_size,
        })
    });

    button.wait_for_low().await?;
    log::info!("Button pressed!");

    spawner.spawn_local(process_events(client, device_info_producer, mount_path))?;*/

    loop {
        // Asynchronously wait for GPIO events, allowing other tasks
        // to run, or the core to sleep.
        button.wait_for_low().await?;
        log::info!("Button pressed!");

        // let contents = fs::read_to_string("/disk/myfile.txt").unwrap_or(String::from("N/A"));
        // log::info!("File content: {}", contents);

        button.wait_for_high().await?;
        log::info!("Button released!");
    }

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // This thread is necessary because the ESP IDF main task thread is running with a very low priority that cannot be raised
    // (lower than the hidden posix thread in `async-io`)
    // As a result, the main thread is constantly starving because of the higher prio `async-io` thread
    //
    // To use async networking IO, make your `main()` minimal by just spawning all work in a new thread
    thread::Builder::new()
        .stack_size(60000)
        .spawn(|| {
            let mut local_executor = LocalPool::new();
            let spawner = local_executor.spawner();
            local_executor.run_until(async move { run_async(spawner).await })
        })
        .unwrap()
        .join()
        .unwrap()
}

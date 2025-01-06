mod api;
mod usb;
mod wifi;

use crate::api::websocket::{WebSocketSession, SessionEvent, ConnectionState};
use crate::usb::msc_device::MSCDevice;
use crate::wifi::session::WifiSession;
use embedded_svc::wifi::AuthMethod;
use embedded_svc::ws::FrameType;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::prelude::Peripherals;
use futures::executor::{LocalPool, LocalSpawner};
use futures::task::LocalSpawnExt;
use std::{thread, time};
use crate::api::processor::process_events;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");
const API_ENDPOINT: &str = env!("API_ENDPOINT");

async fn run_async(spawner: LocalSpawner) -> Result<(), anyhow::Error> {
    log::info!(
        "Start {} - {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let peripherals = Peripherals::take().expect("@@@ Take Peripherals failed");
    let mut wifi = WifiSession::new(SSID, PASSWORD, AuthMethod::WPA2Personal, peripherals.modem)?;
    wifi.connect().await?;
    log::info!("Connected wifi: {:#?}", wifi.get_ip_info());

    let mut msc_device = MSCDevice::new("storage", "/disk");
    msc_device.install()?;

    let mut button = PinDriver::input(peripherals.pins.gpio14)?;
    button.set_pull(Pull::Up)?;

    let mut client = WebSocketSession::new(API_ENDPOINT, time::Duration::from_secs(30));

    // Asynchronously wait for GPIO events, allowing other tasks
    // to run, or the core to sleep.
    button.wait_for_low().await?;
    log::info!("Button pressed!");

    spawner.spawn_local(process_events(client))?;

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

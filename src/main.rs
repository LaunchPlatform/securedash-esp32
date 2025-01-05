mod api;
mod usb;
mod wifi;

use crate::usb::msc_device::MSCDevice;
use crate::wifi::session::WifiSession;
use embedded_svc::wifi::AuthMethod;
use embedded_svc::{http::client::Client as HttpClient, io::Write, utils::io};
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::http::client::EspHttpConnection;
use std::time::Duration;
use std::{fs, thread};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

fn post_chunked_request(
    client: &mut HttpClient<EspHttpConnection>,
    data: &[u8],
) -> anyhow::Result<()> {
    // Prepare payload

    // Prepare headers and URL
    let headers = [("content-type", "text/plain")];
    let url = "https://httpbin.org/post";

    // Send request
    let mut request = client.post(url, &headers)?;
    request.write_all(data)?;
    request.flush()?;
    log::info!("-> CHUNKED POST {}", url);
    let mut response = request.submit()?;

    // Process response
    let status = response.status();
    log::info!("<- {}", status);
    let mut buf = [0u8; 1024];
    let bytes_read = io::try_read_full(&mut response, &mut buf).map_err(|e| e.0)?;
    log::info!("Read {} bytes", bytes_read);
    match std::str::from_utf8(&buf[0..bytes_read]) {
        Ok(body_string) => log::info!(
            "Response body (truncated to {} bytes): {:?}",
            buf.len(),
            body_string
        ),
        Err(e) => log::error!("Error decoding response body: {}", e),
    };

    // Drain the remaining response bytes
    while response.read(&mut buf)? > 0 {}

    Ok(())
}

async fn run_async() -> Result<(), anyhow::Error> {
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

    let config = &esp_idf_svc::http::client::Configuration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };
    let mut client = HttpClient::wrap(EspHttpConnection::new(&config)?);

    loop {
        // Asynchronously wait for GPIO events, allowing other tasks
        // to run, or the core to sleep.
        button.wait_for_low().await?;
        log::info!("Button pressed!");

        let contents = fs::read_to_string("/disk/myfile.txt").unwrap_or(String::from("N/A"));
        log::info!("File content: {}", contents);
        post_chunked_request(&mut client, contents.as_bytes())?;

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
        .spawn(|| block_on(async { run_async().await }))
        .unwrap()
        .join()
        .unwrap()
}

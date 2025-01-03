use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{Input, InputPin, InterruptType, PinDriver, Pull};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{
    esp, esp_partition_find_first, esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_DATA_FAT,
    esp_partition_type_t_ESP_PARTITION_TYPE_DATA, esp_vfs_fat_mount_config_t, tinyusb_config_t,
    tinyusb_driver_install, tinyusb_msc_event_t, tinyusb_msc_spiflash_config_t,
    tinyusb_msc_storage_init_spiflash, tinyusb_msc_storage_mount, tinyusb_msc_storage_unmount,
    wl_handle_t, wl_mount,
};
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, AuthMethod, ClientConfiguration, Configuration, EspWifi};
use std::ffi::CString;
use std::time::Duration;
use std::{fs, thread};

unsafe extern "C" fn storage_mount_changed_cb(event: *mut tinyusb_msc_event_t) {
    log::info!(
        "Mount changed: {}, {}",
        (*event).type_,
        (*event).__bindgen_anon_1.mount_changed_data.is_mounted
    );
}

unsafe extern "C" fn storage_premount_changed_cb(event: *mut tinyusb_msc_event_t) {
    log::info!(
        "Pre-Mount changed: {}, {}",
        (*event).type_,
        (*event).__bindgen_anon_1.mount_changed_data.is_mounted
    );
}

async fn connect_wifi() -> anyhow::Result<AsyncWifi<EspWifi<'static>>> {
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = AsyncWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
        timer_service,
    )?;

    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: "FIXME".try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: "FIXME".try_into().unwrap(),
        channel: None,
        scan_method: Default::default(),
        pmf_cfg: Default::default(),
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start().await?;
    log::info!("Wifi started");

    wifi.connect().await?;
    log::info!("Wifi connected");

    wifi.wait_netif_up().await?;
    log::info!("Wifi netif up");

    Ok(wifi)
}

async fn run_async() -> Result<(), anyhow::Error> {
    log::info!("Hello, world!");

    let wifi = connect_wifi().await?;
    log::info!(
        "Connected wifi: {:#?}",
        wifi.wifi().sta_netif().get_ip_info()?
    );

    let mut handle = wl_handle_t::default();
    let base_path = CString::new("/disk").unwrap();
    let partition_label = CString::new("storage").unwrap();

    let data_partition = unsafe {
        esp_partition_find_first(
            esp_partition_type_t_ESP_PARTITION_TYPE_DATA,
            esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_DATA_FAT,
            partition_label.as_ptr(),
        )
    };

    unsafe {
        wl_mount(data_partition, &mut handle);
    }

    let config_spi = tinyusb_msc_spiflash_config_t {
        wl_handle: handle,
        callback_mount_changed: Some(storage_mount_changed_cb), /* First way to register the callback. This is while initializing the storage. */
        callback_premount_changed: Some(storage_premount_changed_cb),
        mount_config: esp_vfs_fat_mount_config_t {
            format_if_mount_failed: false,
            max_files: 5,
            allocation_unit_size: 0,
            disk_status_check_enable: false,
            use_one_fat: false,
        },
    };
    esp!(unsafe { tinyusb_msc_storage_init_spiflash(&config_spi) })
        .expect("tinyusb_msc_storage_init_spiflash failed");

    // esp!(unsafe { tinyusb_msc_storage_mount(base_path.as_ptr()) })
    //     .expect("failed to usb mount storage;");

    let tusb_cfg = tinyusb_config_t::default();
    unsafe {
        esp!(tinyusb_driver_install(&tusb_cfg)).expect("Failed to install driver");
    }

    log::info!("installed!");
    let peripherals = Peripherals::take()?;

    let mut button = PinDriver::input(peripherals.pins.gpio14)?;
    button.set_pull(Pull::Up)?;
    loop {
        // Asynchronously wait for GPIO events, allowing other tasks
        // to run, or the core to sleep.
        button.wait_for_low().await?;
        log::info!("Button pressed!");

        esp!(unsafe { tinyusb_msc_storage_mount(base_path.as_ptr()) })?;

        let contents = fs::read_to_string("/disk/myfile.txt").unwrap_or(String::from("N/A"));
        log::info!("File content: {}", contents);

        esp!(unsafe { tinyusb_msc_storage_unmount() })?;

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

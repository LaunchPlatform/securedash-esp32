use esp_idf_svc::sys::{
    esp, esp_partition_find_first, esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_DATA_FAT,
    esp_partition_type_t_ESP_PARTITION_TYPE_DATA, esp_vfs_fat_mount_config_t, tinyusb_config_t,
    tinyusb_driver_install, tinyusb_msc_event_t, tinyusb_msc_spiflash_config_t,
    tinyusb_msc_storage_init_spiflash, wl_handle_t, wl_mount,
};
use std::ffi::CString;
use std::thread;
use std::time::Duration;

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

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    log::info!("Hello, world! 2");

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

    loop {
        thread::sleep(Duration::from_millis(1000));
        log::info!("sleep...");
    }
}

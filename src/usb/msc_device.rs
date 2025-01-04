use anyhow::{bail, Context};
use esp_idf_svc::sys::{
    esp, esp_partition_find_first, esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_DATA_FAT,
    esp_partition_type_t_ESP_PARTITION_TYPE_DATA, esp_vfs_fat_mount_config_t, tinyusb_config_t,
    tinyusb_driver_install, tinyusb_msc_event_t, tinyusb_msc_event_type_t,
    tinyusb_msc_event_type_t_TINYUSB_MSC_EVENT_MOUNT_CHANGED,
    tinyusb_msc_event_type_t_TINYUSB_MSC_EVENT_PREMOUNT_CHANGED, tinyusb_msc_spiflash_config_t,
    tinyusb_msc_storage_init_spiflash, tinyusb_msc_storage_mount, wl_handle_t, wl_mount,
};
use std::ffi::CString;

#[derive(Debug, Default)]
pub struct MSCDevice {
    pub partition_label: String,
    pub base_path: String,
    wl_handle: wl_handle_t,
    base_path_c_str: CString,
    partition_label_c_str: CString,
}

fn msc_event_type_to_str(event_type: tinyusb_msc_event_type_t) -> String {
    String::from(match event_type {
        tinyusb_msc_event_type_t_TINYUSB_MSC_EVENT_MOUNT_CHANGED => "MOUNT_CHANGED",
        tinyusb_msc_event_type_t_TINYUSB_MSC_EVENT_PREMOUNT_CHANGED => "PREMOUNT_CHANGED",
        _ => "Unknown",
    })
}

unsafe extern "C" fn storage_mount_changed_cb(event: *mut tinyusb_msc_event_t) {
    log::info!(
        "Mount changed event, type={}, is_mounted={}",
        msc_event_type_to_str((*event).type_),
        (*event).__bindgen_anon_1.mount_changed_data.is_mounted
    );
}

impl MSCDevice {
    pub fn new(partition_label: &str, base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
            partition_label: partition_label.to_string(),
            ..Default::default()
        }
    }

    pub fn install(&mut self) -> anyhow::Result<()> {
        self.partition_label_c_str = CString::new(self.partition_label.as_bytes()).unwrap();
        self.base_path_c_str = CString::new(self.base_path.as_bytes()).unwrap();

        let data_partition = unsafe {
            esp_partition_find_first(
                esp_partition_type_t_ESP_PARTITION_TYPE_DATA,
                esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_DATA_FAT,
                self.partition_label_c_str.as_ptr(),
            )
        };
        if data_partition.is_null() {
            bail!(
                "Failed to find partition with label {}",
                self.partition_label
            );
        }
        esp!(unsafe { wl_mount(data_partition, &mut self.wl_handle) }).with_context(|| {
            format!(
                "Failed to mount partition {} at {}",
                self.partition_label, self.base_path
            )
        })?;

        let config_spi = tinyusb_msc_spiflash_config_t {
            wl_handle: self.wl_handle,
            keep_vfs_fat_mount: true,
            callback_mount_changed: Some(storage_mount_changed_cb),
            callback_premount_changed: Some(storage_mount_changed_cb),
            mount_config: esp_vfs_fat_mount_config_t {
                format_if_mount_failed: false,
                max_files: 5,
                allocation_unit_size: 0,
                disk_status_check_enable: false,
                use_one_fat: false,
            },
        };
        esp!(unsafe { tinyusb_msc_storage_init_spiflash(&config_spi) })
            .with_context(|| "Failed to initialize spiflash")?;

        esp!(unsafe { tinyusb_msc_storage_mount(self.base_path_c_str.as_ptr()) })
            .with_context(|| format!("Failed to mount storage at {}", self.base_path))?;

        let tusb_cfg = tinyusb_config_t::default();
        esp!(unsafe { (tinyusb_driver_install(&tusb_cfg)) })
            .with_context(|| "Failed to install TinyUSB driver")?;

        log::info!("TinyUSB driver installed.");

        Ok(())
    }
}

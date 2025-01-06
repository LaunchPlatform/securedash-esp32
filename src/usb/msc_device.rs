use anyhow::{bail, Context};
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::partition::{EspPartition, EspWlPartition};
use esp_idf_svc::sys::{
    esp, esp_vfs_fat_mount_config_t, tinyusb_config_t, tinyusb_driver_install, tinyusb_msc_event_t,
    tinyusb_msc_event_type_t, tinyusb_msc_event_type_t_TINYUSB_MSC_EVENT_MOUNT_CHANGED,
    tinyusb_msc_event_type_t_TINYUSB_MSC_EVENT_PREMOUNT_CHANGED, tinyusb_msc_spiflash_config_t,
    tinyusb_msc_storage_init_spiflash, tinyusb_msc_storage_mount,
};
use std::ffi::CString;

#[derive(Default)]
pub struct MSCDevice {
    partition_label: String,
    mount_path: String,
    mount_path_c_str: CString,
    high_speed: bool,
    wl_partition: Option<EspWlPartition<EspPartition>>,
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
    pub fn new(partition_label: &str, base_path: &str, high_speed: bool) -> Self {
        Self {
            mount_path: base_path.to_string(),
            partition_label: partition_label.to_string(),
            high_speed,
            ..Default::default()
        }
    }

    pub fn install(&mut self) -> anyhow::Result<()> {
        if self.wl_partition.is_some() {
            bail!("Already installed");
        }
        let partition = Some(
            unsafe { EspPartition::new(&self.partition_label) }?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to find partition with label {:#?}",
                    self.partition_label
                )
            })?,
        );
        let mut wl_partition =
            Some(EspWlPartition::new(partition.unwrap()).with_context(|| {
                format!(
                    "Failed to mount partition {} at {}",
                    self.partition_label, self.mount_path
                )
            })?);

        let config_spi = tinyusb_msc_spiflash_config_t {
            wl_handle: wl_partition.as_mut().unwrap().handle(),
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

        let base_path_c_str = CString::new(self.mount_path.as_bytes()).unwrap();
        esp!(unsafe { tinyusb_msc_storage_mount(base_path_c_str.as_ptr()) })
            .with_context(|| format!("Failed to mount storage at {}", self.mount_path))?;

        let mut tusb_cfg = tinyusb_config_t::default();
        if self.high_speed {
            // TODO:
        }
        esp!(unsafe { tinyusb_driver_install(&tusb_cfg) })
            .with_context(|| "Failed to install TinyUSB driver")?;

        log::info!("TinyUSB driver installed.");

        self.mount_path_c_str = base_path_c_str;
        self.wl_partition = wl_partition;
        Ok(())
    }
}

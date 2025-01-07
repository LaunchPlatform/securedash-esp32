use crate::storage::spiflash::SPIFlashStorage;
use anyhow::Context;
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::sys::{
    esp, tinyusb_config_t, tinyusb_driver_install, tinyusb_msc_event_t, tinyusb_msc_event_type_t,
    tinyusb_msc_event_type_t_TINYUSB_MSC_EVENT_MOUNT_CHANGED,
    tinyusb_msc_event_type_t_TINYUSB_MSC_EVENT_PREMOUNT_CHANGED, tinyusb_msc_spiflash_config_t,
    tinyusb_msc_storage_init_spiflash,
};
use std::fmt::Debug;

pub trait Storage {
    fn config_usb(&self, config: &mut tinyusb_msc_spiflash_config_t) -> anyhow::Result<()>;
}

impl Storage for SPIFlashStorage {
    fn config_usb(&self, config: &mut tinyusb_msc_spiflash_config_t) -> anyhow::Result<()> {
        config.wl_handle = self.handle();
        esp!(unsafe { tinyusb_msc_storage_init_spiflash(config) })
            .with_context(|| "Failed to initialize spiflash for msc storage")?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct MSCDeviceConfig {
    pub high_speed: bool,
}

pub struct MSCDevice {
    config: MSCDeviceConfig,
    storage: Box<dyn Storage>,
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
    pub fn new(config: &MSCDeviceConfig, storage: Box<dyn Storage>) -> Box<Self> {
        Box::new(Self {
            config: config.clone(),
            storage,
        })
    }

    pub fn install(&mut self) -> anyhow::Result<()> {
        let mut config_spi = tinyusb_msc_spiflash_config_t {
            callback_mount_changed: Some(storage_mount_changed_cb),
            callback_premount_changed: Some(storage_mount_changed_cb),
            ..Default::default()
        };
        self.storage.config_usb(&mut config_spi);

        let mut tusb_cfg = tinyusb_config_t::default();
        if self.config.high_speed {
            // TODO:
        }
        esp!(unsafe { tinyusb_driver_install(&tusb_cfg) })
            .with_context(|| "Failed to install TinyUSB driver")?;

        log::info!("TinyUSB driver installed.");
        Ok(())
    }
}

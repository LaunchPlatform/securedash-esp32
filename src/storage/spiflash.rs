use anyhow::{bail, Context};
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::partition::{EspPartition, EspWlPartition};
use esp_idf_svc::sys::wl_handle_t;

#[derive(Debug, Clone)]
pub struct SPIFlashConfig {
    pub partition_label: String,
    pub mount_path: String,
}

pub struct SPIFlashStorage {
    config: SPIFlashConfig,
    wl_partition: Option<EspWlPartition<EspPartition>>,
}

impl SPIFlashStorage {
    pub fn new(config: &SPIFlashConfig) -> Self {
        Self {
            config: config.clone(),
            wl_partition: None,
        }
    }

    pub fn install(&mut self) -> anyhow::Result<()> {
        if self.wl_partition.is_some() {
            bail!("Already installed");
        }
        let partition = Some(
            unsafe { EspPartition::new(&self.config.partition_label) }?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to find partition with label {:#?}",
                    self.config.partition_label
                )
            })?,
        );
        self.wl_partition = Some(EspWlPartition::new(partition.unwrap()).with_context(|| {
            format!(
                "Failed to mount partition {} at {}",
                self.config.partition_label, self.config.mount_path
            )
        })?);
        log::info!(
            "Mount SPI Flash storage with label {} at {}",
            self.config.partition_label,
            self.config.mount_path
        );
        Ok(())
    }
}

impl RawHandle for SPIFlashStorage {
    type Handle = wl_handle_t;

    fn handle(&self) -> Self::Handle {
        self.wl_partition.as_ref().unwrap().handle()
    }
}

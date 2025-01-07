use anyhow::{bail, Context};
use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::partition::{EspPartition, EspWlPartition};
use esp_idf_svc::sys::{esp, ff_diskio_get_drive, wl_handle_t};

#[derive(Debug, Clone)]
pub struct SPIFlashConfig {
    pub partition_label: String,
    pub mount_path: String,
}

pub struct SPIFlashStorage {
    config: SPIFlashConfig,
    wl_partition: Option<EspWlPartition<EspPartition>>,
    mounted_fatfs: Option<MountedFatfs<Fatfs<()>>>,
}

impl SPIFlashStorage {
    pub fn new(config: &SPIFlashConfig) -> Self {
        Self {
            config: config.clone(),
            wl_partition: None,
            mounted_fatfs: None,
        }
    }

    pub fn initialize_partition(&mut self) -> anyhow::Result<()> {
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

    pub fn mount(&mut self, mount_path: &str, max_fds: usize) -> anyhow::Result<()> {
        if self.wl_partition.is_none() {
            bail!("Partition not initialized yet");
        }
        if self.mounted_fatfs.is_some() {
            bail!("File system already mounted");
        }
        let mut drive: u8 = 0;
        esp!(unsafe { ff_diskio_get_drive(&mut drive) })?;
        let fatfs = unsafe { Fatfs::new_wl_part(drive, self.handle()) }?;
        self.mounted_fatfs = Some(MountedFatfs::mount(fatfs, mount_path, max_fds)?);
        Ok(())
    }
}

impl RawHandle for SPIFlashStorage {
    type Handle = wl_handle_t;

    fn handle(&self) -> Self::Handle {
        self.wl_partition.as_ref().unwrap().handle()
    }
}

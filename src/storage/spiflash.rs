use anyhow::{bail, Context};
use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::partition::{EspPartition, EspWlPartition};
use esp_idf_svc::sys::{esp, ff_diskio_get_drive, wl_handle_t};

pub struct SPIFlashStorage {
    wl_partition: Option<EspWlPartition<EspPartition>>,
    mounted_fatfs: Option<MountedFatfs<Fatfs<()>>>,
}

impl SPIFlashStorage {
    pub fn new() -> Self {
        Self {
            wl_partition: None,
            mounted_fatfs: None,
        }
    }

    pub fn initialize_partition(&mut self, partition_label: &str) -> anyhow::Result<()> {
        if self.wl_partition.is_some() {
            bail!("Already installed");
        }
        let partition = Some(
            unsafe { EspPartition::new(partition_label) }?.ok_or_else(|| {
                anyhow::anyhow!("Failed to find partition with label {:#?}", partition_label)
            })?,
        );
        self.wl_partition = Some(EspWlPartition::new(partition.unwrap()).with_context(|| {
            format!(
                "Failed to create WL partition for partition with label {}",
                partition_label,
            )
        })?);
        log::info!(
            "Initialized SPI Flash WL partition label {}",
            partition_label,
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
        // TODO: handle file system not formatted issue.
        //       looks like esp-idf-svc didn't implement error handling at this moment:
        //       https://github.com/esp-rs/esp-idf-svc/blob/6453bedd5967ebc3bcb7d240492a0d5164e818c2/src/io.rs#L130
        // TODO: the f_mount called with async mode, it means the actual mount will only happen in
        //       a file operation. Maybe we should leave file sys formatting to the end-user
        //       instead? as it's a dangerous operation anyway....
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

use anyhow::bail;
use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver};
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::sys::{esp, ff_diskio_get_drive};

pub struct SDCardStorage<'a> {
    sd_card_driver: Option<SdCardDriver<SdMmcHostDriver<'a>>>,
    mounted_fatfs: Option<MountedFatfs<Fatfs<SdCardDriver<SdMmcHostDriver<'a>>>>>,
}

impl<'a> SDCardStorage<'a> {
    pub fn new() -> Self {
        Self {
            sd_card_driver: None,
            mounted_fatfs: None,
        }
    }
    pub fn install_driver(&mut self, peripherals: &mut Peripherals) -> anyhow::Result<()> {
        if self.mounted_fatfs.is_some() {
            bail!("File system already mounted");
        }
        if self.sd_card_driver.is_some() {
            bail!("Driver already installed");
        }
        let pins = &mut peripherals.pins;
        self.sd_card_driver = Some(SdCardDriver::new_mmc(
            SdMmcHostDriver::new_4bits(
                peripherals.sdmmc0,
                pins.gpio35,
                pins.gpio36,
                pins.gpio37,
                pins.gpio38,
                pins.gpio33,
                pins.gpio34,
                None::<gpio::AnyIOPin>,
                None::<gpio::AnyIOPin>,
                &SdMmcHostConfiguration::new(),
            )?,
            &SdCardConfiguration::new(),
        )?);
        Ok(())
    }

    pub fn mount(&mut self, mount_path: &str, max_fds: usize) -> anyhow::Result<()> {
        if self.mounted_fatfs.is_some() {
            bail!("File system already mounted");
        } else {
            if self.sd_card_driver.is_none() {
                bail!("SD card driver not installed yet");
            }
        }
        let mut drive: u8 = 0;
        esp!(unsafe { ff_diskio_get_drive(&mut drive) })?;
        let fatfs = unsafe { Fatfs::new_sdcard(drive, self.sd_card_driver.take().unwrap()) }?;
        self.mounted_fatfs = Some(MountedFatfs::mount(fatfs, mount_path, max_fds)?);
        Ok(())
    }
}

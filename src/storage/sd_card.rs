use anyhow::bail;
use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::gpio::{Gpio33, Gpio34, Gpio35, Gpio36, Gpio37, Gpio38};
use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver, SDMMC1};
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::sys::{esp, ff_diskio_get_drive, sdmmc_card_t};

pub struct SDCardPeripherals {
    pub slot: SDMMC1,
    pub cmd: Gpio35,
    pub clk: Gpio36,
    pub d0: Gpio37,
    pub d1: Gpio38,
    pub d2: Gpio33,
    pub d3: Gpio34,
}

#[macro_export]
macro_rules! sd_peripherals {
    ($x:expr ) => {
        SDCardPeripherals {
            slot: $x.sdmmc1,
            cmd: $x.pins.gpio35,
            clk: $x.pins.gpio36,
            d0: $x.pins.gpio37,
            d1: $x.pins.gpio38,
            d2: $x.pins.gpio33,
            d3: $x.pins.gpio34,
        }
    };
}

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
    pub fn install_driver(&mut self, peripherals: SDCardPeripherals) -> anyhow::Result<()> {
        if self.mounted_fatfs.is_some() {
            bail!("File system already mounted");
        }
        if self.sd_card_driver.is_some() {
            bail!("Driver already installed");
        }
        let mut host_config = SdMmcHostConfiguration::new();
        // Notice: the dev board use external pullups
        // TODO: make this configurable?
        host_config.enable_internal_pullups = false;
        self.sd_card_driver = Some(SdCardDriver::new_mmc(
            SdMmcHostDriver::new_4bits(
                peripherals.slot,
                peripherals.cmd,
                peripherals.clk,
                peripherals.d0,
                peripherals.d1,
                peripherals.d2,
                peripherals.d3,
                None::<gpio::AnyIOPin>,
                None::<gpio::AnyIOPin>,
                &host_config,
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
        let fatfs = Fatfs::new_sdcard(drive, self.sd_card_driver.take().unwrap())?;
        self.mounted_fatfs = Some(MountedFatfs::mount(fatfs, mount_path, max_fds)?);
        Ok(())
    }

    pub fn card(&self) -> Option<&sdmmc_card_t> {
        self.sd_card_driver.as_ref().map(SdCardDriver::card)
    }
}

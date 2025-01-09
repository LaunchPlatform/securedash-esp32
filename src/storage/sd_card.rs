use anyhow::bail;
use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::gpio::{Gpio33, Gpio34, Gpio35, Gpio36, Gpio37, Gpio38};
use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver, SDMMC1};
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::sys::{
    esp, ff_diskio_get_drive, sdmmc_card_t, SDMMC_FREQ_52M,
};
use std::borrow::Borrow;
use std::mem::replace;

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

pub enum SDCardState<'a> {
    Init,
    DriverInstalled {
        driver: SdCardDriver<SdMmcHostDriver<'a>>,
    },
    Mounted {
        card: sdmmc_card_t,
        mounted_fatfs: MountedFatfs<Fatfs<SdCardDriver<SdMmcHostDriver<'a>>>>,
    },
}

pub struct SDCardStorage<'a> {
    state: SDCardState<'a>,
}

impl<'a> SDCardStorage<'a> {
    pub fn new() -> Self {
        Self {
            state: SDCardState::Init,
        }
    }
    pub fn install_driver(&mut self, peripherals: SDCardPeripherals) -> anyhow::Result<()> {
        match &self.state {
            SDCardState::DriverInstalled { .. } => {
                bail!("Driver already installed");
            }
            SDCardState::Mounted { .. } => {
                bail!("File system already mounted");
            }
            _ => {}
        }
        let mut host_config = SdMmcHostConfiguration::new();
        // Notice: the dev board use external pullups
        // TODO: make this configurable?
        host_config.enable_internal_pullups = false;
        let mut card_config = SdCardConfiguration::new();
        // TODO: make this config
        card_config.speed_khz = SDMMC_FREQ_52M;
        self.state = SDCardState::DriverInstalled {
            driver: SdCardDriver::new_mmc(
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
                &card_config,
            )?,
        };
        Ok(())
    }

    pub fn mount(&mut self, mount_path: &str, max_fds: usize) -> anyhow::Result<()> {
        self.state = match replace(&mut self.state, SDCardState::Init) {
            SDCardState::Init => {
                bail!("Driver not installed yet");
            }
            SDCardState::Mounted { .. } => {
                bail!("File system already mounted");
            }
            SDCardState::DriverInstalled { mut driver } => {
                let mut drive: u8 = 0;
                esp!(unsafe { ff_diskio_get_drive(&mut drive) })?;
                let card = driver.card().clone();
                let fatfs = Fatfs::new_sdcard(drive, driver)?;
                SDCardState::Mounted {
                    card,
                    mounted_fatfs: MountedFatfs::mount(fatfs, mount_path, max_fds)?,
                }
            }
        };
        Ok(())
    }

    pub fn card(&self) -> Option<&sdmmc_card_t> {
        match &self.state {
            SDCardState::Init => None,
            SDCardState::DriverInstalled { driver } => Some(driver.card()),
            SDCardState::Mounted { card, .. } => Some(card),
        }
    }
}

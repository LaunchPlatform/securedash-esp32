use anyhow::bail;
use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::gpio::{Gpio33, Gpio34, Gpio35, Gpio36, Gpio37, Gpio38};
use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver, SDMMC1};
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::sys::{esp, ff_diskio_get_drive, sdmmc_card_t};
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::mem::replace;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

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

pub struct SDDriverHolder<'a> {
    driver: Rc<RefCell<SdCardDriver<SdMmcHostDriver<'a>>>>,
}

impl<'a> Clone for SDDriverHolder<'a> {
    fn clone(&self) -> Self {
        Self {
            driver: Rc::clone(&self.driver),
        }
    }
}

impl<'a> SDDriverHolder<'a> {
    fn new(driver: SdCardDriver<SdMmcHostDriver<'a>>) -> Self {
        Self {
            driver: Rc::new(RefCell::new(driver)),
        }
    }

    fn card(&self) -> &sdmmc_card_t {
        self.driver.deref().borrow().card()
    }
}

impl<'a> Borrow<SdCardDriver<SdMmcHostDriver<'a>>> for SDDriverHolder<'a> {
    fn borrow(&self) -> &SdCardDriver<SdMmcHostDriver<'a>> {
        self.driver.deref().borrow().deref()
    }
}

impl<'a> BorrowMut<SdCardDriver<SdMmcHostDriver<'a>>> for SDDriverHolder<'a> {
    fn borrow_mut(self: &mut SDDriverHolder<'a>) -> &mut SdCardDriver<SdMmcHostDriver<'a>> {
        self.driver.deref().borrow_mut().deref_mut()
    }
}

pub enum SDCardState<'a> {
    Init,
    DriverInstalled {
        driver: SDDriverHolder<'a>,
    },
    Mounted {
        driver: SDDriverHolder<'a>,
        mounted_fatfs: MountedFatfs<Fatfs<SDDriverHolder<'a>>>,
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
        self.state = SDCardState::DriverInstalled {
            driver: SDDriverHolder::new(SdCardDriver::new_mmc(
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
            )?),
        };
        Ok(())
    }

    pub fn mount(&mut self, mount_path: &str, max_fds: usize) -> anyhow::Result<()> {
        match replace(&mut self.state, SDCardState::Init) {
            SDCardState::Init => {
                bail!("Driver not installed yet");
            }
            SDCardState::Mounted { .. } => {
                bail!("File system already mounted");
            }
            SDCardState::DriverInstalled { mut driver } => {
                let mut drive: u8 = 0;
                esp!(unsafe { ff_diskio_get_drive(&mut drive) })?;
                let fatfs = Fatfs::new_sdcard(drive, driver.clone())?;
                SDCardState::Mounted {
                    driver,
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
            SDCardState::Mounted { driver, .. } => Some(driver.card()),
            _ => None,
        }
    }
}

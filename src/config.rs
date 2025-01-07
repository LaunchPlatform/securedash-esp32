use serde::Deserialize;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Read;

#[derive(Default, Debug, Deserialize)]
pub enum AuthMethod {
    None,
    WEP,
    WPA,
    #[default]
    WPA2Personal,
    WPAWPA2Personal,
    WPA2Enterprise,
    WPA3Personal,
    WPA2WPA3Personal,
    WAPIPersonal,
}

#[derive(Deserialize)]
pub struct Wifi {
    ssid: String,
    auth_method: AuthMethod,
    password: Option<String>,
}

impl Debug for Wifi {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wifi")
            .field("ssid", &self.ssid)
            .field("auth_method", &self.auth_method)
            .field("password", &"****")
            .finish()
    }
}

#[derive(Debug, Deserialize)]
pub struct Api {
    endpoint: String,
}

#[derive(Debug, Deserialize)]
pub struct Usb {
    high_speed: bool,
}

impl Default for Usb {
    fn default() -> Self {
        Self { high_speed: true }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    wifi: Wifi,
    api: Api,
    usb: Usb,
}

impl Config {
    pub fn read(file_path: &str) -> anyhow::Result<Self> {
        let mut config_str = String::new();
        File::open(file_path)?.read_to_string(&mut config_str)?;
        Ok(toml::from_str(&*config_str)?)
    }
}

use serde::Deserialize;
use std::fmt::{Debug, Formatter};

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
struct Wifi {
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
struct Api {
    endpoint: String,
}

#[derive(Debug, Deserialize)]
struct Usb {
    high_speed: bool,
}

impl Default for Usb {
    fn default() -> Self {
        Self { high_speed: true }
    }
}

#[derive(Debug, Deserialize)]
struct Config {
    wifi: Wifi,
    api: Api,
    usb: Usb,
}

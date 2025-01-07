use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::ipv4;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};

#[derive(Debug, Clone)]
pub struct WifiConfig {
    ssid: String,
    password: Option<String>,
    auth_method: Option<AuthMethod>,
}

pub struct WifiSession<'a> {
    async_wifi: AsyncWifi<EspWifi<'a>>,
}

impl<'a> WifiSession<'a> {
    pub(crate) fn new(config: &WifiConfig, modem: Modem) -> anyhow::Result<Self> {
        let sys_loop = EspSystemEventLoop::take()?;
        let timer_service = EspTaskTimerService::new()?;
        let nvs = EspDefaultNvsPartition::take()?;
        let mut async_wifi = AsyncWifi::wrap(
            EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
            sys_loop,
            timer_service,
        )?;

        let mut client_config = ClientConfiguration {
            ssid: config.ssid.as_str().try_into().unwrap(),
            ..Default::default()
        };
        if let Some(password) = &config.password {
            client_config.password = password.as_str().try_into().unwrap();
        }
        if let Some(auth_method) = &config.auth_method {
            client_config.auth_method = *auth_method;
        }
        let wifi_configuration: Configuration = Configuration::Client(client_config);
        async_wifi.set_configuration(&wifi_configuration)?;
        log::info!("Initialized wifi");
        Ok(Self { async_wifi })
    }

    pub async fn connect(&mut self) -> anyhow::Result<()> {
        self.async_wifi.start().await?;
        log::info!("Wifi started");

        self.async_wifi.connect().await?;
        log::info!("Wifi connected");

        self.async_wifi.wait_netif_up().await?;
        log::info!("Wifi netif up");

        Ok(())
    }

    pub fn get_ip_info(&self) -> Result<ipv4::IpInfo, EspError> {
        self.async_wifi.wifi().sta_netif().get_ip_info()
    }
}

use esp_idf_svc::sys::{sdmmc_card_t, sdmmc_host_t};
use std::fmt;
use std::fmt::{Debug, Formatter};

pub struct CardInfo<'a> {
    card: &'a sdmmc_card_t,
}

impl<'a> CardInfo<'a> {
    pub(crate) fn new(card: &'a sdmmc_card_t) -> CardInfo<'a> {
        Self { card }
    }
}

pub struct HostInfo<'a> {
    host: &'a sdmmc_host_t,
}
impl<'a> HostInfo<'a> {
    fn new(host: &'a sdmmc_host_t) -> HostInfo<'a> {
        Self { host }
    }
}

impl Debug for HostInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("sdmmc_host_t")
            .field("flags", &self.host.flags)
            .field("slot", &self.host.slot)
            .field("max_freq_khz", &self.host.max_freq_khz)
            .field("command_timeout_ms", &self.host.command_timeout_ms)
            .finish()
    }
}

impl Debug for CardInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("sdmmc_card_t")
            .field("host", &HostInfo::new(&self.card.host))
            .field("ocr", &self.card.ocr)
            .field("max_freq_khz", &self.card.max_freq_khz)
            .field("real_freq_khz", &self.card.real_freq_khz)
            .finish()
    }
}

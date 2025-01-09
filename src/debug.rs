use esp_idf_svc::sys::{sdmmc_card_t, sdmmc_csd_t, sdmmc_ext_csd_t, sdmmc_host_t};
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
            .field("input_delay_phase", &self.host.input_delay_phase)
            .finish()
    }
}
pub struct CSDInfo<'a> {
    csd: &'a sdmmc_csd_t,
}
impl<'a> CSDInfo<'a> {
    fn new(csd: &'a sdmmc_csd_t) -> CSDInfo<'a> {
        Self { csd }
    }
}
impl Debug for CSDInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("sdmmc_csd_t")
            .field("csd_ver", &self.csd.csd_ver)
            .field("mmc_ver", &self.csd.mmc_ver)
            .field("capacity", &self.csd.capacity)
            .field("sector_size", &self.csd.sector_size)
            .field("read_block_len", &self.csd.read_block_len)
            .field("card_command_class", &self.csd.card_command_class)
            .field("tr_speed", &self.csd.tr_speed)
            .finish()
    }
}

pub struct ExtCSDInfo<'a> {
    ext_csd: &'a sdmmc_ext_csd_t,
}
impl<'a> ExtCSDInfo<'a> {
    fn new(ext_csd: &'a sdmmc_ext_csd_t) -> ExtCSDInfo<'a> {
        Self { ext_csd }
    }
}
impl Debug for ExtCSDInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("sdmmc_ext_csd_t")
            .field("rev", &self.ext_csd.rev)
            .field("power_class", &self.ext_csd.power_class)
            .field("erase_mem_state", &self.ext_csd.erase_mem_state)
            .field("sec_feature", &self.ext_csd.sec_feature)
            .finish()
    }
}

impl Debug for CardInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("sdmmc_card_t")
            .field("host", &HostInfo::new(&self.card.host))
            .field("csd", &CSDInfo::new(&self.card.csd))
            .field("ext_csd", &ExtCSDInfo::new(&self.card.ext_csd))
            .field("ocr", &self.card.ocr)
            .field("rca", &self.card.rca)
            .field("max_freq_khz", &self.card.max_freq_khz)
            .field("real_freq_khz", &self.card.real_freq_khz)
            .finish()
    }
}

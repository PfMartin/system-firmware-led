use anyhow::{anyhow, bail, Result};
use embedded_svc::wifi::{
    AccessPointConfiguration, AuthMethod, ClientConfiguration, Configuration,
};
use esp_idf_hal::{modem::Modem, peripheral::Peripheral};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::BlockingWifi, wifi::EspWifi,
};
use heapless::String;
use log::info;

pub fn connect_to_wifi(
    ssid: &'static str,
    pwd: &'static str,
    modem: impl Peripheral<P = Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> Result<Box<EspWifi<'static>>> {
    info!("Starting wifi connection process");

    check_credentials_not_empty(ssid, pwd)?;

    let nvs = EspDefaultNvsPartition::take()?;
    let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

    let default_conf = Configuration::Client(ClientConfiguration::default());
    wifi.set_configuration(&default_conf)?;

    info!("Starting WiFi");
    wifi.start()?;

    info!("Scanning...");
    let ap_infos = wifi.scan()?;
    let ap_match = ap_infos.into_iter().find(|a| a.ssid == ssid);

    let channel = if let Some(ap_match) = ap_match {
        info!(
            "Found configured access point {} on channel {}",
            ssid, ap_match.channel
        );
        Some(ap_match.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    let wifi_ssid = String::<32>::try_from(ssid).map_err(|e| anyhow!("Error: {:?}", e))?;
    let wifi_psk = String::<64>::try_from(pwd).map_err(|e| anyhow!("Error: {:?}", e))?;

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: wifi_ssid,
            password: wifi_psk,
            channel,
            auth_method: AuthMethod::WPA2Personal,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: String::<32>::try_from("aptest").unwrap(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    info!("Connecting WiFi...");
    wifi.connect()?;

    info!("Waiting for DHCP lease...");
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Connected to WiFi: {:?}", ip_info);

    Ok(Box::new(esp_wifi))
}

fn check_credentials_not_empty(ssid: &'static str, pwd: &'static str) -> Result<()> {
    if ssid.is_empty() {
        bail!("Missing WiFi name");
    }

    if pwd.is_empty() {
        bail!("Missing WiFi password");
    }

    Ok(())
}

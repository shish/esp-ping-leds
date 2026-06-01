use esp_idf_svc::{
    handle::RawHandle,
    ipv4::Ipv4Addr,
    wifi::{AuthMethod, BlockingWifi, EspWifi},
};
use std::time::Duration;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use crate::{debug_lights, BootStage};

pub fn connect_wifi(
    ws2812: &mut Ws2812Esp32Rmt,
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    ssid: &str,
    password: &str,
) -> anyhow::Result<()> {
    log::info!("Wifi starting, target: {}...", ssid);
    debug_lights(ws2812, BootStage::WifiStarting)?;

    // Set hostname before starting wifi
    log::info!("Setting hostname to ping-leds");
    unsafe {
        let hostname = std::ffi::CString::new("ping-leds").unwrap();
        esp_idf_svc::sys::esp_netif_set_hostname(
            wifi.wifi().sta_netif().handle(),
            hostname.as_ptr(),
        );
    }

    wifi.start()?;

    let auth_method = scan_wifi(wifi, ssid, password)?;
    let wifi_configuration =
        esp_idf_svc::wifi::Configuration::Client(esp_idf_svc::wifi::ClientConfiguration {
            ssid: heapless::String::<32>::try_from(ssid).unwrap(),
            auth_method,
            password: heapless::String::<64>::try_from(password).unwrap(),
            ..Default::default()
        });
    wifi.set_configuration(&wifi_configuration)?;

    log::info!("Connecting...");
    debug_lights(ws2812, BootStage::WifiConnecting)?;
    wifi.connect()?;

    log::info!("Waiting for DHCP...");
    debug_lights(ws2812, BootStage::WifiDhcp)?;
    wifi.wait_netif_up()?;

    log::info!("Print DHCP info...");
    debug_lights(ws2812, BootStage::WifiPrintInfo)?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("Wifi DHCP info: {:?}", ip_info);

    debug_lights(ws2812, BootStage::WifiComplete)?;
    Ok(())
}

pub fn scan_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    ssid: &str,
    password: &str,
) -> anyhow::Result<AuthMethod> {
    log::info!("Scanning...");
    let guessed_auth = if password.is_empty() {
        AuthMethod::None
    } else {
        AuthMethod::WPA2Personal
    };
    match wifi.scan() {
        Ok(aps) => {
            aps.iter().for_each(|i| {
                println!(
                    "AP: {} {:?} {} {} {:?}",
                    i.ssid, i.bssid, i.channel, i.signal_strength, i.auth_method
                )
            });
            let ours = aps.into_iter().find(|a| a.ssid == ssid);
            if let Some(ours) = ours {
                if let Some(auth_method) = ours.auth_method {
                    log::info!(
                        "Found configured AP {} with auth method {:?}",
                        ssid,
                        auth_method
                    );
                    Ok(auth_method)
                } else {
                    log::info!("Found configured AP {} with unknown auth method", ssid);
                    Ok(guessed_auth)
                }
            } else {
                log::debug!("Configured AP {} not found", ssid);
                Ok(guessed_auth)
            }
        }
        Err(e) => {
            log::error!("Wifi scan failed: {}", e);
            Ok(guessed_auth)
        }
    }
}

pub fn ping(host: Ipv4Addr, timeout: Duration) -> anyhow::Result<Option<Duration>> {
    let mut pinger = esp_idf_svc::ping::EspPing::new(0);
    let conf = esp_idf_svc::ping::Configuration {
        interval: Duration::from_secs(0),
        timeout,
        ..Default::default()
    };
    let summary = pinger.ping(host, &conf)?;
    if summary.received != summary.transmitted {
        Ok(None)
    } else {
        Ok(Some(summary.time / summary.transmitted))
    }
}

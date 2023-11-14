use esp_idf_hal::{delay::FreeRtos, peripherals::Peripherals};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    ipv4::Ipv4Addr,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, EspWifi},
};
use esp_idf_sys as _;
use smart_leds::SmartLedsWrite;
use smart_leds::RGB;
use std::{collections::VecDeque, time::Duration};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const WIFI_SSID: &str = "Wokwi-GUEST";
const WIFI_PASS: &str = "";
const HOST: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);
const MAX_HEALTHY_DURATION: Duration = Duration::from_millis(200);
const LED_STRIP_DURATION: Duration = Duration::from_secs(60);
const LED_COUNT: u32 = 16;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Root startup...");
    log::info!("Get peripherals");
    let peripherals = Peripherals::take()?;
    log::info!("Get system event loop");
    let sysloop = EspSystemEventLoop::take()?;
    log::info!("Get NVS partition");
    let nvs = EspDefaultNvsPartition::take()?;
    log::info!("About to initialize WiFi (SSID: {})", WIFI_SSID);
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
    )?;

    loop {
        match connect_wifi(&mut wifi, WIFI_SSID, WIFI_PASS) {
            Ok(_) => log::info!("Wifi ok"),
            Err(e) => {
                log::error!("Wifi connection failed: {}", e);
                log::info!("Restarting in 10s...");
                FreeRtos::delay_ms(10000);
                continue;
            }
        }
        match main_loop() {
            Ok(_) => unreachable!(),
            Err(e) => {
                log::error!("Major Error: {}", e);
                log::info!("Restarting in 10s...");
                FreeRtos::delay_ms(10000);
            }
        }
    }
}

fn connect_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    wifi_ssid: &str,
    wifi_password: &str,
) -> anyhow::Result<()> {
    let client_config = esp_idf_svc::wifi::ClientConfiguration {
        ssid: wifi_ssid.into(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: wifi_password.into(),
        channel: None,
    };
    let wifi_configuration = esp_idf_svc::wifi::Configuration::Client(client_config);
    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    log::info!("Wifi started");
    wifi.connect()?;
    log::info!("Wifi connected");
    wifi.wait_netif_up()?;
    log::info!("Wifi netif up");
    Ok(())
}

fn main_loop() -> anyhow::Result<()> {
    let mut ws2812 = Ws2812Esp32Rmt::new(0, 6)?;
    let time_per_led = LED_STRIP_DURATION / LED_COUNT;
    let mut samples: VecDeque<Option<Duration>> = VecDeque::with_capacity((LED_COUNT + 1) as usize);

    loop {
        // let sample = Some(Duration::from_millis(42));
        // let sample = Some(Duration::from_millis(
        //     unsafe { esp_idf_sys::esp_random() % 250 } as u64,
        // ));
        let sample = ping(HOST)?;
        log::info!("Sample: {:?}", sample);

        samples.push_front(sample);
        if samples.len() > LED_COUNT as usize {
            samples.pop_back();
        }
        let pixels = samples
            .clone()
            .into_iter()
            .map(|ms| ms2rgb(ms, MAX_HEALTHY_DURATION));
        ws2812.write(pixels)?;
        FreeRtos::delay_ms(time_per_led.as_millis() as u32);
    }
}

fn ping(host: Ipv4Addr) -> anyhow::Result<Option<Duration>> {
    let mut pinger = esp_idf_svc::ping::EspPing::new(0);
    let conf = esp_idf_svc::ping::Configuration {
        interval: Duration::from_secs(0),
        timeout: MAX_HEALTHY_DURATION * 5,
        ..Default::default()
    };
    let summary = pinger.ping(host, &conf)?;
    if summary.received != summary.transmitted {
        Ok(None)
    } else {
        Ok(Some(summary.time / summary.transmitted))
    }
}

/// Converts a given value in milliseconds to an RGB color value.
///
/// # Arguments
///
/// * `sample` - How long the ping took, or None for "timeout"
/// * `max` - Durations larger than this should be considered problems
///
/// # Returns
///
/// An RGB<u8> value representing the converted color.
fn ms2rgb(sample: Option<Duration>, max: Duration) -> RGB<u8> {
    let max = max.as_millis() as u32;
    match sample {
        None => RGB::new(255, 0, 0),
        Some(d) => {
            let ms = d.as_millis() as u32;
            if ms <= 1 {
                RGB::new(0, 255, 0)
            } else if ms > max {
                RGB::new(127, 0, 0)
            } else {
                let r = (f64::log10(ms as f64) * (255.0 / f64::log10(max as f64))) as u8;
                RGB::new(r, 255, 0)
            }
        }
    }
}

#[cfg(test)]
mod test_ms2rgb {
    use super::*;

    const TEST_MAX: Duration = Duration::from_millis(100);

    #[test]
    fn timeout_returns_red() {
        assert_eq!(ms2rgb(None, TEST_MAX), RGB::new(255, 0, 0));
    }

    #[test]
    fn fast_returns_green() {
        assert_eq!(ms2rgb(Some(0), TEST_MAX), RGB::new(0, 255, 0));
    }

    #[test]
    fn slow_returns_yellow() {
        assert_eq!(ms2rgb(Some(50), TEST_MAX), RGB::new(128, 255, 0));
    }

    #[test]
    fn very_slow_returns_red() {
        assert_eq!(ms2rgb(Some(200), TEST_MAX), RGB::new(127, 0, 0));
    }
}

mod network;
mod rgb;

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::FreeRtos, peripherals::Peripherals},
    ipv4::Ipv4Addr,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};
use smart_leds::SmartLedsWrite;
use smart_leds::RGB;
use std::{collections::VecDeque, time::Duration};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const WIFI_SSID: Option<&str> = std::option_env!("WIFI_SSID");
const WIFI_PASS: Option<&str> = std::option_env!("WIFI_PASS");
const PING_HOST: Option<&str> = std::option_env!("PING_HOST");
const MAX_HEALTHY_DURATION: Duration = Duration::from_millis(100);
const LED_STRIP_DURATION: Duration = Duration::from_secs(30 * 60);
const LED_COUNT: u32 = 24;
const RESTART_SECONDS: u32 = 3;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches
    // to the runtime implemented by esp-idf-sys might not link properly.
    // See https://github.com/esp-rs/esp-idf-template/issues/71
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

    log::info!("Allocate wifi");
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
    )?;
    let wifi_ssid = WIFI_SSID.unwrap_or("Wokwi-GUEST");
    let wifi_pass = WIFI_PASS.unwrap_or("");

    log::info!("LED setup");
    // gpio6 for C3 mini, wokwi, esp32-c3-devkit-rust-1
    // gpio13 for ESP-WROOM-32
    let mut ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio6)?;
    log::info!("LED boot debug lights");
    debug_lights(&mut ws2812, 1)?;

    match network::connect_wifi(&mut ws2812, &mut wifi, wifi_ssid, wifi_pass) {
        Ok(_) => log::info!("Wifi ok"),
        Err(e) => {
            log::error!("Wifi connection failed: {}", e);
            log::info!("Restarting in {}s...", RESTART_SECONDS);
            FreeRtos::delay_ms(RESTART_SECONDS * 1000);
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    }
    let ping_host = if PING_HOST.is_some() {
        PING_HOST.unwrap().parse::<Ipv4Addr>()?
    } else {
        wifi.wifi().sta_netif().get_ip_info()?.subnet.gateway
    };
    match main_loop(ping_host, ws2812) {
        Ok(_) => unreachable!(),
        Err(e) => {
            log::error!("Major Error: {}", e);
            log::info!("Restarting in {}s...", RESTART_SECONDS);
            FreeRtos::delay_ms(RESTART_SECONDS * 1000);
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    }
}

pub fn debug_lights(ws2812: &mut Ws2812Esp32Rmt, stage: u32) -> anyhow::Result<()> {
    ws2812.write((0..LED_COUNT).map(|n| {
        if n < stage {
            RGB::new(100, 50, 75)
        } else {
            RGB::new(0, 0, 50)
        }
    }))?;
    Ok(())
}

fn main_loop(ping_host: Ipv4Addr, mut ws2812: Ws2812Esp32Rmt) -> anyhow::Result<()> {
    log::info!("Main loop...");
    let time_per_led = LED_STRIP_DURATION / LED_COUNT;
    let mut samples: VecDeque<Option<Duration>> = VecDeque::with_capacity((LED_COUNT + 1) as usize);
    loop {
        // let sample = Some(Duration::from_millis(42));
        // let sample = Some(Duration::from_millis(
        //     unsafe { esp_idf_sys::esp_random() % 250 } as u64,
        // ));
        let sample = network::ping(ping_host, MAX_HEALTHY_DURATION * 5)?;
        log::info!("Sample: {:?}", sample);

        samples.push_front(sample);
        if samples.len() > LED_COUNT as usize {
            samples.pop_back();
        }
        let pixels = samples
            .clone()
            .into_iter()
            .map(|ms| rgb::ms2rgb(ms, MAX_HEALTHY_DURATION));
        ws2812.write(pixels)?;
        FreeRtos::delay_ms(time_per_led.as_millis() as u32);
    }
}

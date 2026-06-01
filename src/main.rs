mod config;
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
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use config::Config;

#[derive(Debug, Clone, Copy)]
pub enum BootStage {
    Startup = 1,
    WifiStarting = 2,
    WifiConnecting = 3,
    WifiDhcp = 4,
    WifiPrintInfo = 5,
    WifiComplete = 6,
}

const WIFI_SSID: Option<&str> = std::option_env!("WIFI_SSID");
const WIFI_PASS: Option<&str> = std::option_env!("WIFI_PASS");
const PING_HOST: Option<&str> = std::option_env!("PING_HOST");
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
    debug_lights(&mut ws2812, BootStage::Startup)?;

    match network::connect_wifi(&mut ws2812, &mut wifi, wifi_ssid, wifi_pass) {
        Ok(_) => log::info!("Wifi ok"),
        Err(e) => {
            log::error!("Wifi connection failed: {}", e);
            log::info!("Restarting in {}s...", RESTART_SECONDS);
            FreeRtos::delay_ms(RESTART_SECONDS * 1000);
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    }
    let ping_host_str = if PING_HOST.is_some() {
        PING_HOST.unwrap().to_string()
    } else {
        wifi.wifi()
            .sta_netif()
            .get_ip_info()?
            .subnet
            .gateway
            .to_string()
    };

    log::info!("Creating config...");
    let config = Config::new_shared(
        Duration::from_millis(100),   // max_healthy_duration
        127,                          // led_brightness
        true,                         // led_enabled
        ping_host_str,                // ping_host
        Duration::from_secs(30 * 60), // led_strip_duration (30 minutes)
        24,                           // led_count
    );

    match main_loop(config, ws2812) {
        Ok(_) => unreachable!(),
        Err(e) => {
            log::error!("Major Error: {}", e);
            log::info!("Restarting in {}s...", RESTART_SECONDS);
            FreeRtos::delay_ms(RESTART_SECONDS * 1000);
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    }
}

pub fn debug_lights(ws2812: &mut Ws2812Esp32Rmt, stage: BootStage) -> anyhow::Result<()> {
    let stage_num = stage as u32;
    let led_count = 24; // Use a constant here for boot-time debug
    ws2812.write((0..led_count).map(|n| {
        if n < stage_num {
            RGB::new(100, 50, 75)
        } else {
            RGB::new(0, 0, 50)
        }
    }))?;
    Ok(())
}

fn main_loop(config: Arc<Mutex<Config>>, mut ws2812: Ws2812Esp32Rmt) -> anyhow::Result<()> {
    log::info!("Main loop...");

    let mut samples: VecDeque<Option<Duration>> = VecDeque::new();
    let mut elapsed_since_sample = Duration::MAX;

    loop {
        // Read config values for this iteration
        let (ping_host, max_healthy_duration, led_brightness, led_enabled, led_count, time_per_led) = {
            let cfg = config.lock().unwrap();
            let time_per_led = cfg.led_strip_duration / cfg.led_count;
            (
                cfg.ping_host.clone(),
                cfg.max_healthy_duration,
                cfg.led_brightness,
                cfg.led_enabled,
                cfg.led_count,
                time_per_led,
            )
        };

        // Check if it's time to take a new sample
        if elapsed_since_sample >= time_per_led {
            let ping_host_addr = ping_host.parse::<Ipv4Addr>()?;
            let sample = network::ping(ping_host_addr, max_healthy_duration * 5)?;
            log::info!("Sample: {:?}", sample);
            samples.push_front(sample);
            if samples.len() > led_count as usize {
                samples.pop_back();
            }
            elapsed_since_sample = Duration::ZERO;
        }

        // Update the pixels
        let pixels: Vec<RGB<u8>> = if led_enabled {
            samples
                .clone()
                .into_iter()
                .map(|ms| rgb::ms2rgb(ms, max_healthy_duration, led_brightness))
                .chain(
                    std::iter::repeat(smart_leds::RGB8::new(0, 0, led_brightness / 4))
                        .take(led_count as usize - samples.len()),
                )
                .collect()
        } else {
            vec![RGB::new(0, 0, 0); led_count as usize]
        };
        ws2812.write(pixels.into_iter())?;

        // Sleep until the next loop
        let loop_delay = Duration::from_secs(1);
        FreeRtos::delay_ms(loop_delay.as_millis() as u32);
        elapsed_since_sample += loop_delay;
    }
}

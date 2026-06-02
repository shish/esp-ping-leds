use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Configuration for the LED ping monitor
#[derive(Debug, Clone)]
pub struct Config {
    /// Minimum duration for a ping to be considered perfectly healthy (solid green)
    pub min_healthy_duration: Duration,
    /// Maximum duration for a ping to be considered healthy
    pub max_healthy_duration: Duration,
    /// LED brightness (0-255)
    pub led_brightness: u8,
    /// Whether LEDs are enabled
    pub led_enabled: bool,
    /// Host to ping (as a string, will be resolved to IP)
    pub ping_host: String,
    /// Total duration the LED strip represents
    pub led_strip_duration: Duration,
    /// Number of LEDs in the strip
    pub led_count: u32,
}

impl Config {
    /// Create a new Config with the given values
    pub fn new(
        min_healthy_duration: Duration,
        max_healthy_duration: Duration,
        led_brightness: u8,
        led_enabled: bool,
        ping_host: String,
        led_strip_duration: Duration,
        led_count: u32,
    ) -> Self {
        Self {
            min_healthy_duration,
            max_healthy_duration,
            led_brightness,
            led_enabled,
            ping_host,
            led_strip_duration,
            led_count,
        }
    }

    /// Create a new Config wrapped in Arc<Mutex<>> for shared access
    pub fn new_shared(
        min_healthy_duration: Duration,
        max_healthy_duration: Duration,
        led_brightness: u8,
        led_enabled: bool,
        ping_host: String,
        led_strip_duration: Duration,
        led_count: u32,
    ) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new(
            min_healthy_duration,
            max_healthy_duration,
            led_brightness,
            led_enabled,
            ping_host,
            led_strip_duration,
            led_count,
        )))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_healthy_duration: Duration::from_millis(10),
            max_healthy_duration: Duration::from_millis(50),
            led_brightness: 127,
            led_enabled: true,
            ping_host: String::new(), // Will be set to gateway by default
            led_strip_duration: Duration::from_secs(30 * 60),
            led_count: 24,
        }
    }
}

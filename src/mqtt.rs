use crate::config::Config;
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration, QoS};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MQTT_URL: Option<&str> = std::option_env!("MQTT_URL");

/// MQTT client wrapper for Home Assistant integration
pub struct MqttManager {
    client: EspMqttClient<'static>,
    config: Arc<Mutex<Config>>,
    device_id: String,
    device_path: String,
    publish_pending: Arc<Mutex<bool>>,
}

impl MqttManager {
    /// Create a new MQTT manager if MQTT_URL environment variable is set
    pub fn new(config: Arc<Mutex<Config>>, mac_address: [u8; 6]) -> anyhow::Result<Option<Self>> {
        let broker_url = match MQTT_URL {
            Some(url) => url,
            None => {
                log::info!("MQTT_URL not set, MQTT disabled");
                return Ok(None);
            }
        };

        log::info!("Connecting to MQTT broker: {}...", broker_url);

        let mqtt_config = MqttClientConfiguration {
            keep_alive_interval: Some(Duration::from_secs(60)),
            reconnect_timeout: Some(Duration::from_secs(10)),
            ..Default::default()
        };

        let (client, mut connection) = EspMqttClient::new(&broker_url, &mqtt_config)?;

        log::info!("MQTT client created, spawning connection handler");

        // Flag to signal when state should be published
        let publish_pending = Arc::new(Mutex::new(false));

        // Generate device ID from MAC address: ping_leds_aabbccddeeff
        let device_id = format!(
            "ping_leds_{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            mac_address[0],
            mac_address[1],
            mac_address[2],
            mac_address[3],
            mac_address[4],
            mac_address[5]
        );
        log::info!("Device ID: {}", device_id);

        let mut manager = Self {
            client,
            config: config.clone(),
            device_path: format!("homeassistant/device/{}", device_id),
            device_id: device_id.clone(),
            publish_pending,
        };

        // Spawn connection handler thread
        let config_clone = config.clone();
        let device_path_clone = manager.device_path.clone();
        let publish_pending_clone = manager.publish_pending.clone();
        std::thread::Builder::new()
            .stack_size(8192)
            .spawn(move || {
                Self::connection_handler(
                    &mut connection,
                    config_clone,
                    device_path_clone,
                    publish_pending_clone,
                );
            })?;

        // Wait briefly for connection
        std::thread::sleep(Duration::from_millis(1000));

        // Send discovery messages
        manager.send_discovery_messages()?;

        // Wait a bit for subscription to complete
        std::thread::sleep(Duration::from_millis(500));

        // Publish initial state
        manager.publish_state()?;

        log::info!("MQTT manager initialized");
        Ok(Some(manager))
    }

    /// Handle incoming MQTT messages
    fn connection_handler(
        connection: &mut EspMqttConnection,
        config: Arc<Mutex<Config>>,
        device_path: String,
        publish_pending: Arc<Mutex<bool>>,
    ) {
        log::info!("MQTT connection handler started");

        while let Ok(event) = connection.next() {
            use esp_idf_svc::mqtt::client::EventPayload;

            match event.payload() {
                EventPayload::Connected(_) => {
                    log::info!("MQTT Connected");
                }
                EventPayload::Disconnected => {
                    log::warn!("MQTT Disconnected");
                }
                EventPayload::Subscribed(id) => {
                    log::info!("MQTT Subscribed to topic ID: {}", id);
                }
                EventPayload::Received {
                    id,
                    topic,
                    data,
                    details: _,
                } => {
                    if let Some(topic_str) = topic {
                        if let Ok(payload) = std::str::from_utf8(data) {
                            log::info!("MQTT Received [{}] {}: {}", id, topic_str, payload);
                            Self::handle_command(
                                topic_str,
                                payload,
                                &config,
                                &device_path,
                                &publish_pending,
                            );
                        }
                    }
                }
                EventPayload::Error(err) => {
                    log::error!("MQTT Error: {:?}", err);
                }
                _ => {}
            }
        }

        log::warn!("MQTT connection handler exited");
    }

    /// Handle incoming MQTT command messages
    fn handle_command(
        topic: &str,
        payload: &str,
        config: &Arc<Mutex<Config>>,
        device_path: &str,
        publish_pending: &Arc<Mutex<bool>>,
    ) {
        let light_cmd_topic = format!("{}/light/set", device_path);
        let min_healthy_topic = format!("{}/min_healthy_duration/set", device_path);
        let max_healthy_topic = format!("{}/max_healthy_duration/set", device_path);
        let led_strip_topic = format!("{}/led_strip_duration/set", device_path);
        let led_count_topic = format!("{}/led_count/set", device_path);
        let ping_host_topic = format!("{}/ping_host/set", device_path);

        if topic == light_cmd_topic {
            #[derive(Deserialize)]
            struct LightCommand {
                state: Option<String>,
                brightness: Option<u8>,
            }

            // Parse JSON payload
            match serde_json::from_str::<LightCommand>(payload) {
                Ok(cmd) => {
                    let mut state_changed = false;
                    if let Ok(mut cfg) = config.lock() {
                        // Handle ON/OFF state
                        if let Some(state) = cmd.state {
                            if state == "ON" {
                                log::info!("Turning light ON");
                                cfg.led_enabled = true;
                                if cfg.led_brightness == 0 {
                                    cfg.led_brightness = 127; // Default to medium brightness if it was 0
                                }
                                state_changed = true;
                            } else if state == "OFF" {
                                log::info!("Turning light OFF");
                                cfg.led_enabled = false;
                                state_changed = true;
                            }
                        }

                        // Handle brightness
                        if let Some(brightness) = cmd.brightness {
                            log::info!("Setting brightness to {}", brightness);
                            cfg.led_brightness = brightness;
                            // If setting brightness > 0, turn on
                            if brightness > 0 {
                                cfg.led_enabled = true;
                            }
                            state_changed = true;
                        }
                    }

                    // Signal that state should be published
                    if state_changed {
                        if let Ok(mut pending) = publish_pending.lock() {
                            *pending = true;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to parse light command: {}", e);
                }
            }
        } else if topic == min_healthy_topic {
            if let Ok(ms) = payload.trim().parse::<u64>() {
                if let Ok(mut cfg) = config.lock() {
                    cfg.min_healthy_duration = Duration::from_millis(ms);
                    log::info!("Set min_healthy_duration to {}ms", ms);
                    if let Ok(mut pending) = publish_pending.lock() {
                        *pending = true;
                    }
                }
            } else {
                log::error!("Failed to parse min_healthy_duration: {}", payload);
            }
        } else if topic == max_healthy_topic {
            if let Ok(ms) = payload.trim().parse::<u64>() {
                if let Ok(mut cfg) = config.lock() {
                    cfg.max_healthy_duration = Duration::from_millis(ms);
                    log::info!("Set max_healthy_duration to {}ms", ms);
                    if let Ok(mut pending) = publish_pending.lock() {
                        *pending = true;
                    }
                }
            } else {
                log::error!("Failed to parse max_healthy_duration: {}", payload);
            }
        } else if topic == led_strip_topic {
            if let Ok(secs) = payload.trim().parse::<u64>() {
                if let Ok(mut cfg) = config.lock() {
                    cfg.led_strip_duration = Duration::from_secs(secs);
                    log::info!("Set led_strip_duration to {}s", secs);
                    if let Ok(mut pending) = publish_pending.lock() {
                        *pending = true;
                    }
                }
            } else {
                log::error!("Failed to parse led_strip_duration: {}", payload);
            }
        } else if topic == led_count_topic {
            if let Ok(count) = payload.trim().parse::<u32>() {
                if let Ok(mut cfg) = config.lock() {
                    cfg.led_count = count;
                    log::info!("Set led_count to {}", count);
                    if let Ok(mut pending) = publish_pending.lock() {
                        *pending = true;
                    }
                }
            } else {
                log::error!("Failed to parse led_count: {}", payload);
            }
        } else if topic == ping_host_topic {
            if let Ok(mut cfg) = config.lock() {
                cfg.ping_host = payload.trim().to_string();
                log::info!("Set ping_host to {}", cfg.ping_host);
                if let Ok(mut pending) = publish_pending.lock() {
                    *pending = true;
                }
            }
        } else {
            log::warn!("Received command for unknown topic: {}", topic);
        }
    }

    /// Send Home Assistant discovery message for all entities
    fn send_discovery_messages(&mut self) -> anyhow::Result<()> {
        log::info!("Sending Home Assistant discovery message");

        let cfg = self.config.lock().unwrap();

        // Single discovery message with all components
        let discovery_config = serde_json::json!({
            "device": {
                "identifiers": [&self.device_id],
                "name": "Ping LEDs",
                "manufacturer": "Shish",
                "model": "ESP32-C3 Ping Monitor"
            },
            "origin": {
                "name": "esp-ping-leds-firmware"
            },
            "components": {
                "leds": {
                    "platform": "light",
                    "name": "LEDs",
                    "unique_id": format!("{}_leds", self.device_id),
                    "object_id": format!("{}_leds", self.device_id),
                    "state_topic": format!("{}/light/state", self.device_path),
                    "command_topic": format!("{}/light/set", self.device_path),
                    "brightness": true,
                    "brightness_scale": 255,
                    "schema": "json"
                },
                "min_healthy_duration": {
                    "platform": "number",
                    "name": "Min Healthy Duration",
                    "unique_id": format!("{}_min_healthy_duration", self.device_id),
                    "object_id": format!("{}_min_healthy_duration", self.device_id),
                    "state_topic": format!("{}/min_healthy_duration/state", self.device_path),
                    "command_topic": format!("{}/min_healthy_duration/set", self.device_path),
                    "unit_of_measurement": "ms",
                    "min": 1,
                    "max": 1000,
                    "step": 1,
                    "mode": "box"
                },
                "max_healthy_duration": {
                    "platform": "number",
                    "name": "Max Healthy Duration",
                    "unique_id": format!("{}_max_healthy_duration", self.device_id),
                    "object_id": format!("{}_max_healthy_duration", self.device_id),
                    "state_topic": format!("{}/max_healthy_duration/state", self.device_path),
                    "command_topic": format!("{}/max_healthy_duration/set", self.device_path),
                    "unit_of_measurement": "ms",
                    "min": 1,
                    "max": 1000,
                    "step": 1,
                    "mode": "box"
                },
                "led_strip_duration": {
                    "platform": "number",
                    "name": "LED Strip Duration",
                    "unique_id": format!("{}_led_strip_duration", self.device_id),
                    "object_id": format!("{}_led_strip_duration", self.device_id),
                    "state_topic": format!("{}/led_strip_duration/state", self.device_path),
                    "command_topic": format!("{}/led_strip_duration/set", self.device_path),
                    "unit_of_measurement": "s",
                    "min": 60,
                    "max": 7200,
                    "step": 60,
                    "mode": "box"
                },
                "led_count": {
                    "platform": "number",
                    "name": "LED Count",
                    "unique_id": format!("{}_led_count", self.device_id),
                    "object_id": format!("{}_led_count", self.device_id),
                    "state_topic": format!("{}/led_count/state", self.device_path),
                    "command_topic": format!("{}/led_count/set", self.device_path),
                    "min": 1,
                    "max": 300,
                    "step": 1,
                    "mode": "box"
                },
                "ping_host": {
                    "platform": "text",
                    "name": "Ping Host",
                    "unique_id": format!("{}_ping_host", self.device_id),
                    "object_id": format!("{}_ping_host", self.device_id),
                    "state_topic": format!("{}/ping_host/state", self.device_path),
                    "command_topic": format!("{}/ping_host/set", self.device_path),
                    "mode": "text"
                }
            }
        })
        .to_string();

        log::info!("Sending discovery config to {}/config", self.device_path);

        // Send single discovery message
        self.client.enqueue(
            &format!("{}/config", self.device_path),
            QoS::AtLeastOnce,
            true,
            discovery_config.as_bytes(),
        )?;

        // Subscribe to all command topics
        self.client
            .subscribe(&format!("{}/light/set", self.device_path), QoS::AtLeastOnce)?;
        self.client.subscribe(
            &format!("{}/min_healthy_duration/set", self.device_path),
            QoS::AtLeastOnce,
        )?;
        self.client.subscribe(
            &format!("{}/max_healthy_duration/set", self.device_path),
            QoS::AtLeastOnce,
        )?;
        self.client.subscribe(
            &format!("{}/led_strip_duration/set", self.device_path),
            QoS::AtLeastOnce,
        )?;
        self.client.subscribe(
            &format!("{}/led_count/set", self.device_path),
            QoS::AtLeastOnce,
        )?;
        self.client.subscribe(
            &format!("{}/ping_host/set", self.device_path),
            QoS::AtLeastOnce,
        )?;

        drop(cfg); // Release the lock

        log::info!("Discovery message sent");
        Ok(())
    }

    /// Publish current state to MQTT
    pub fn publish_state(&mut self) -> anyhow::Result<()> {
        let cfg = self.config.lock().unwrap();

        self.client.enqueue(
            &format!("{}/light/state", self.device_path),
            QoS::AtLeastOnce,
            true,
            serde_json::json!({
                "state": if cfg.led_enabled { "ON" } else { "OFF" },
                "brightness": cfg.led_brightness
            })
            .to_string()
            .as_bytes(),
        )?;
        self.client.enqueue(
            &format!("{}/min_healthy_duration/state", self.device_path),
            QoS::AtLeastOnce,
            true,
            cfg.min_healthy_duration.as_millis().to_string().as_bytes(),
        )?;
        self.client.enqueue(
            &format!("{}/max_healthy_duration/state", self.device_path),
            QoS::AtLeastOnce,
            true,
            cfg.max_healthy_duration.as_millis().to_string().as_bytes(),
        )?;
        self.client.enqueue(
            &format!("{}/led_strip_duration/state", self.device_path),
            QoS::AtLeastOnce,
            true,
            cfg.led_strip_duration.as_secs().to_string().as_bytes(),
        )?;
        self.client.enqueue(
            &format!("{}/led_count/state", self.device_path),
            QoS::AtLeastOnce,
            true,
            cfg.led_count.to_string().as_bytes(),
        )?;
        self.client.enqueue(
            &format!("{}/ping_host/state", self.device_path),
            QoS::AtLeastOnce,
            true,
            cfg.ping_host.as_bytes(),
        )?;

        Ok(())
    }

    /// Periodically publish state (call this from main loop)
    pub fn periodic_publish(&mut self) -> anyhow::Result<()> {
        // Check if there's a pending state change to publish
        let should_publish = if let Ok(mut pending) = self.publish_pending.lock() {
            if *pending {
                *pending = false;
                true
            } else {
                false
            }
        } else {
            false
        };

        if should_publish {
            self.publish_state()?;
        }

        Ok(())
    }
}

pub mod commands;
pub mod ping;
mod status;

use std::{
    collections::BTreeMap,
    fmt::Display,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};

use poise::serenity_prelude::{ChannelId, GuildId, MessageId, RoleId, Timestamp};
use serde::{Deserialize, Serialize};
use tokio::sync::{OnceCell, RwLock};

use crate::status::{DEFAULT_DOWN_MESSAGE, DEFAULT_UP_MESSAGE};

pub const DEFAULT_RESOURCE_NAME: &str = "BYOND";
pub const DEFAULT_RESOURCE_ADDR: &str = "hub.byond.com";
pub const DEFAULT_ATTEMPTS_BEFORE_NOTIFICATION: u8 = 3;
pub const DEFAULT_TIMEOUT_SECS: u64 = 5;
pub const DEFAULT_INTERVAL_BETWEEN_ATTEMPTS_SECS: u64 = 10;

pub const DEFAULT_SAVEDATA_PATH: &str = "Data.toml";
pub const DEFAULT_CONFIG_PATH: &str = "Config.toml";
pub const DEFAULT_LOG_PATH: &str = "debug.log";

// Yeah, it's hardcoded. Change it there, if you fork.
pub const DEFAULT_REPOSITORY: &str = "https://github.com/VladOS-0/discord-watchdog";

pub static THIS_RUN_START: OnceCell<Timestamp> = OnceCell::const_new();

#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub enum ResourceStatus {
    Up,
    Down,
    #[default]
    Unknown,
}

impl Display for ResourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceStatus::Up => write!(f, "Up"),
            ResourceStatus::Down => write!(f, "Down"),
            ResourceStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

pub type Data = Arc<AppData>;

#[derive(Default, Debug)]
pub struct AppData {
    status: RwLock<ResourceStatus>,
    used_messages: RwLock<BTreeMap<GuildId, ServerUsedMessages>>,
    attempts_before_notification: AtomicU8,
    last_status_change: RwLock<Timestamp>,
    config: RwLock<Config>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SavedData {
    status: ResourceStatus,
    used_messages: BTreeMap<GuildId, ServerUsedMessages>,
    attempts_before_notification: u8,
    last_status_change: Timestamp,
    pub config: Config,
}

impl SavedData {
    pub async fn load_from_file<T: AsRef<Path>>(data_path: &T) -> anyhow::Result<Option<Self>> {
        let data_file = tokio::fs::read_to_string(data_path).await;
        if let Ok(data_string) = data_file {
            Ok(Some(toml::from_str::<SavedData>(&data_string).map_err(
                |err| {
                    anyhow::Error::msg(format!(
                        "Malformed saved data in {}: {}",
                        data_path.as_ref().to_string_lossy(),
                        err
                    ))
                },
            )?))
        } else {
            let err = data_file.unwrap_err();
            if let std::io::ErrorKind::NotFound = err.kind() {
                Ok(None)
            } else {
                Err(anyhow::Error::msg(format!(
                    "Failed to open {}: {}",
                    data_path.as_ref().to_string_lossy(),
                    err
                )))
            }
        }
    }
    pub async fn save_to_file<T: AsRef<Path>>(&self, config_path: &T) -> anyhow::Result<()> {
        let serialized_string = toml::to_string_pretty(self).map_err(|err| {
            anyhow::Error::msg(format!(
                "Broken serialization of SaveData: got {}, while serializing {:?}",
                err, self
            ))
        })?;
        tokio::fs::write(config_path, serialized_string.as_bytes())
            .await
            .map_err(|err| {
                anyhow::Error::msg(format!(
                    "failed to write SaveData to {}: {}",
                    config_path.as_ref().to_string_lossy(),
                    err
                ))
            })?;
        Ok(())
    }
    pub async fn load_into(&self, data: &AppData) {
        *data.status.write().await = self.status;
        *data.used_messages.write().await = self.used_messages.clone();
        data.attempts_before_notification
            .store(self.attempts_before_notification, Ordering::Relaxed);
        *data.last_status_change.write().await = self.last_status_change;
        *data.config.write().await = self.config.clone();
    }
    pub async fn load_from(data: &AppData) -> Self {
        Self {
            status: (*data.status.read().await),
            used_messages: (*data.used_messages.read().await).clone(),
            attempts_before_notification: data.attempts_before_notification.load(Ordering::Relaxed),
            last_status_change: (*data.last_status_change.read().await),
            config: (*data.config.read().await).clone(),
        }
    }
}

/// IDs of messages that were created by the bot to inform users about resource status changes
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct ServerUsedMessages {
    status: Option<MessageId>,
}

impl ServerUsedMessages {
    pub fn new(status: Option<MessageId>) -> Self {
        Self { status }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Config {
    master_server: Option<GuildId>,
    max_servers: usize,
    ping_config: PingConfig,
    server_configs: BTreeMap<GuildId, ServerConfig>,
}

impl Config {
    pub async fn load_from_file<T: AsRef<Path>>(config_path: &T) -> anyhow::Result<Option<Self>> {
        let config_file = tokio::fs::read_to_string(config_path).await;
        if let Ok(config_string) = config_file {
            Ok(Some(toml::from_str::<Config>(&config_string).map_err(
                |err| {
                    anyhow::Error::msg(format!(
                        "Malformed config data in {}: {}",
                        config_path.as_ref().to_string_lossy(),
                        err
                    ))
                },
            )?))
        } else {
            let err = config_file.unwrap_err();
            if let std::io::ErrorKind::NotFound = err.kind() {
                Ok(None)
            } else {
                Err(anyhow::Error::msg(format!(
                    "Failed to open {}: {}",
                    config_path.as_ref().to_string_lossy(),
                    err
                )))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PingConfig {
    resource_name: String,
    resource_addr: String,
    required_attempts_before_notification: u8,
    timeout: Duration,
    interval_between_attempts: Duration,
}

impl Default for PingConfig {
    fn default() -> Self {
        Self {
            resource_name: DEFAULT_RESOURCE_NAME.to_string(),
            resource_addr: DEFAULT_RESOURCE_ADDR.to_string(),
            required_attempts_before_notification: DEFAULT_ATTEMPTS_BEFORE_NOTIFICATION,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            interval_between_attempts: Duration::from_secs(DEFAULT_INTERVAL_BETWEEN_ATTEMPTS_SECS),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    name: String,
    channel: Option<ChannelId>,
    role_to_notify: Option<RoleId>,
    up_message: String,
    down_message: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "Noname server".to_string(),
            channel: None,
            role_to_notify: None,
            up_message: DEFAULT_UP_MESSAGE.to_string(),
            down_message: DEFAULT_DOWN_MESSAGE.to_string(),
        }
    }
}

impl ServerConfig {
    fn with_name(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

pub async fn save_data<T: AsRef<AppData>>(data: T) {
    let new_saved_data = SavedData::load_from(data.as_ref()).await;
    if let Err(err) = new_saved_data.save_to_file(&DEFAULT_SAVEDATA_PATH).await {
        log::error!(
            "Failed to save SaveData to {}: {}",
            &DEFAULT_SAVEDATA_PATH,
            err
        )
    } else {
        log::info!("Saved SaveData to {}", DEFAULT_SAVEDATA_PATH);
    }
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

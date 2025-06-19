use std::sync::{Arc, atomic::Ordering};

use poise::serenity_prelude::{Channel, CreateEmbed, CreateMessage, Http, Timestamp};

use crate::{Data, ResourceStatus, save_data};

pub async fn update_status(status: ResourceStatus, data: Data, http: Arc<Http>) {
    let old_status_lock = data.status.read().await;
    let old_status = *old_status_lock;
    drop(old_status_lock);
    if status == old_status {
        data.attempts_before_notification
            .store(0, Ordering::Relaxed);
        return;
    }

    // Status changed
    let config = data.config.read().await;
    let required_attempts_before_notification = config.required_attempts_before_notification;
    drop(config);

    if data
        .attempts_before_notification
        .fetch_add(1, Ordering::Relaxed)
        >= required_attempts_before_notification
    {
        log::info!("Changed status from {} to {}", old_status, status);
        data.attempts_before_notification
            .store(0, Ordering::Relaxed);
        *data.status.write().await = status;
        *data.last_status_change.write().await = Timestamp::now();
        notify_status_change(old_status, status, data.clone(), http.clone()).await;
        save_data(&data).await;
    }
}

pub async fn notify_status_change(
    old_status: ResourceStatus,
    new_status: ResourceStatus,
    data: Data,
    http: Arc<Http>,
) {
    let config_lock = data.config.read().await;
    let resource_name = config_lock.resource_name.clone();
    let role_ping = match config_lock.role_to_notify {
        Some(id) => {
            format!("<@&{}>", id)
        }
        None => "fellas".to_string(),
    };
    let optimistic = config_lock.optimistic;
    let channel_id = config_lock.channel;
    let channel = match channel_id {
        Some(id) => {
            let channel_result = http.get_channel(id).await;
            if let Ok(channel) = channel_result {
                channel
            } else {
                log::error!(
                    "Failed to fetch channel: {}. Notification aborted.",
                    channel_result.unwrap_err()
                );
                return;
            }
        }
        None => {
            log::error!("No notification channel specified. Notification aborted.");
            return;
        }
    };
    drop(config_lock);

    match (old_status, new_status) {
        (_, ResourceStatus::Unknown) => {
            update_embed(&resource_name, new_status, data, channel, http.clone()).await;
        }
        (ResourceStatus::Unknown, _) => {
            update_embed(&resource_name, new_status, data, channel, http.clone()).await;
        }
        (ResourceStatus::Up, ResourceStatus::Down) => {
            let mut message = "Nevermind, it's dead again. Boowomp :sob:.".to_string();
            if optimistic {
                message = format!("{} has gone down, {}!", resource_name, role_ping);
            }
            let send_result = channel
                .id()
                .send_message(http.clone(), CreateMessage::new().content(message))
                .await;
            match send_result {
                Ok(message) => {
                    log::info!("Sent new down message with id {}", message.id);
                }
                Err(err) => {
                    log::error!("Failed to send new down message: {}", err);
                    return;
                }
            }
            update_embed(&resource_name, new_status, data, channel, http.clone()).await;
        }
        (ResourceStatus::Down, ResourceStatus::Up) => {
            let mut message = format!("{} is back online, {}!", resource_name, role_ping);
            if optimistic {
                message = "Okay, it's up again.".to_string();
            }
            let send_result = channel
                .id()
                .send_message(http.clone(), CreateMessage::new().content(message))
                .await;
            match send_result {
                Ok(message) => {
                    log::info!("Sent new up message with id {}", message.id);
                }
                Err(err) => {
                    log::error!("Failed to send new up message: {}", err);
                    return;
                }
            }
            update_embed(&resource_name, new_status, data, channel, http).await;
        }
        _ => unreachable!(),
    }
}

pub async fn update_embed(
    resource_name: &str,
    new_status: ResourceStatus,
    data: Data,
    channel: Channel,
    http: Arc<Http>,
) {
    let mut messages = data.used_messages.write().await;
    let addr = data.config.read().await.resource_addr.clone();
    let last_status_change = data.last_status_change.read().await;

    match messages.status {
        Some(id) => {
            let message_result = http.get_message(channel.id(), id).await;
            match message_result {
                Ok(message) => {
                    let deletion_result = message.delete(http.clone()).await;
                    if let Err(err) = deletion_result {
                        log::error!("Failed to delete old status message: {}", err);
                        return;
                    } else {
                        log::info!("Deleted old status message");
                    }
                    let send_result = channel
                        .id()
                        .send_message(
                            http,
                            CreateMessage::new().embed(generate_embed(
                                resource_name,
                                new_status,
                                addr,
                                *last_status_change,
                            )),
                        )
                        .await;
                    match send_result {
                        Ok(message) => {
                            messages.status = Some(message.id);
                            log::info!("Sent new status message with id {}", message.id);
                        }
                        Err(err) => {
                            log::error!("Failed to send new status message: {}", err);
                        }
                    }
                }
                Err(err) => {
                    log::warn!(
                        "Failed to fetch status message because of: {}. Creating new one...",
                        err
                    );
                    let send_result = channel
                        .id()
                        .send_message(
                            http,
                            CreateMessage::new().embed(generate_embed(
                                resource_name,
                                new_status,
                                addr,
                                *last_status_change,
                            )),
                        )
                        .await;
                    match send_result {
                        Ok(message) => {
                            messages.status = Some(message.id);
                            log::info!("Sent new status message with id {}", message.id);
                        }
                        Err(err) => {
                            log::error!("Failed to send new status message: {}", err);
                        }
                    }
                }
            }
        }
        None => {
            log::info!("No status message detected. Creating new one...",);
            let send_result = channel
                .id()
                .send_message(
                    http,
                    CreateMessage::new().embed(generate_embed(
                        resource_name,
                        new_status,
                        addr,
                        *last_status_change,
                    )),
                )
                .await;
            match send_result {
                Ok(message) => {
                    messages.status = Some(message.id);
                    log::info!("Sent new status message with id {}", message.id);
                }
                Err(err) => {
                    log::error!("Failed to send new status message: {}", err);
                }
            }
        }
    }
}

pub fn generate_embed(
    resource_name: &str,
    new_status: ResourceStatus,
    addr: String,
    last_status_change: Timestamp,
) -> CreateEmbed {
    let mut new_embed = CreateEmbed::new();
    match new_status {
        ResourceStatus::Up => {
            new_embed = new_embed
                .color((21, 250, 59))
                .title(format!("{} is online!", resource_name));
        }
        ResourceStatus::Down => {
            new_embed = new_embed
                .color((220, 23, 30))
                .title(format!("{} is offline!", resource_name));
        }
        ResourceStatus::Unknown => {
            new_embed = new_embed
                .color((215, 187, 10))
                .title(format!("{} status is unknown...", resource_name))
                .description("Some kind of error occured. Notify maintainers!");
        }
    };
    new_embed = new_embed.fields(vec![
        (
            "Since",
            format!("<t:{}:R>", last_status_change.unix_timestamp()),
            false,
        ),
        ("Address", addr, false),
    ]);
    new_embed
}

use std::sync::{Arc, atomic::Ordering};

use poise::serenity_prelude::{
    Channel, CreateEmbed, CreateMessage, GuildId, Http, RoleId, Timestamp,
};

use crate::{Data, ResourceStatus, ServerUsedMessages, save_data};

pub const DEFAULT_UP_MESSAGE: &str = "%%RESOURCE%% is back online, %%ROLE%%!";
pub const DEFAULT_DOWN_MESSAGE: &str = "Nevermind, it's dead again. Boowomp :sob:.";

const ROLE_FALLBACK_STRING: &str = "people";

const TEMPLATE_RESOURCE_NAME: &str = "%%RESOURCE%%";
const TEMPLATE_ROLE_PING: &str = "%%ROLE%%";

pub async fn update_status(status: ResourceStatus, data: Data, http: Arc<Http>) {
    let old_status = data.status.read().await.to_owned();
    if status == old_status {
        data.attempts_before_notification
            .store(0, Ordering::Relaxed);
        return;
    }

    // Status changed
    let config = data.config.read().await;
    let required_attempts_before_notification =
        config.ping_config.required_attempts_before_notification;
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
    let resource_name = config_lock.ping_config.resource_name.clone();
    let addr = data.config.read().await.ping_config.resource_addr.clone();
    let last_status_change = data.last_status_change.read().await.to_owned();

    let embed = generate_embed(resource_name.as_str(), new_status, addr, last_status_change);

    for (server_id, server_config) in &config_lock.server_configs {
        let role_id = server_config.role_to_notify;
        let channel_id = server_config.channel;
        let channel = match channel_id {
            Some(id) => {
                let channel_result = http.clone().get_channel(id).await;
                if let Ok(channel) = channel_result {
                    channel
                } else {
                    log::warn!(
                        "[server {}] Failed to fetch channel: {}. Notification aborted.",
                        server_id,
                        channel_result.unwrap_err()
                    );
                    continue;
                }
            }
            None => {
                log::warn!(
                    "[server {}] No notification channel specified. Notification aborted.",
                    server_id
                );
                continue;
            }
        };

        match (old_status, new_status) {
            (_, ResourceStatus::Unknown) => {
                update_embed(*server_id, &embed, data.clone(), channel, http.clone()).await;
            }
            (ResourceStatus::Unknown, _) => {
                update_embed(*server_id, &embed, data.clone(), channel, http.clone()).await;
            }
            (ResourceStatus::Up, ResourceStatus::Down) => {
                let message: String = replace_templates(
                    server_config.down_message.as_str(),
                    &resource_name,
                    &role_id,
                );
                let send_result = channel
                    .id()
                    .send_message(http.clone(), CreateMessage::new().content(message))
                    .await;
                match send_result {
                    Ok(message) => {
                        log::info!(
                            "[server {}] Sent new down message with id {}",
                            server_id,
                            message.id
                        );
                    }
                    Err(err) => {
                        log::error!(
                            "[server {}] Failed to send new down message: {}",
                            server_id,
                            err
                        );
                        continue;
                    }
                }
                update_embed(*server_id, &embed, data.clone(), channel, http.clone()).await;
            }
            (ResourceStatus::Down, ResourceStatus::Up) => {
                let message: String =
                    replace_templates(server_config.up_message.as_str(), &resource_name, &role_id);
                let send_result = channel
                    .id()
                    .send_message(http.clone(), CreateMessage::new().content(message))
                    .await;
                match send_result {
                    Ok(message) => {
                        log::info!(
                            "[server {}] Sent new up message with id {}",
                            server_id,
                            message.id
                        );
                    }
                    Err(err) => {
                        log::error!(
                            "[server {}] Failed to send new up message: {}",
                            server_id,
                            err
                        );
                        continue;
                    }
                }
                update_embed(*server_id, &embed, data.clone(), channel, http.clone()).await;
            }
            _ => unreachable!(),
        }
    }

    drop(config_lock);
}

pub async fn update_embed(
    server_id: GuildId,
    embed: &CreateEmbed,
    data: Data,
    channel: Channel,
    http: Arc<Http>,
) {
    // let's just pray this staff will not cause any deadlocks
    log::trace!("Acquiring message_lock in update_embed...");
    let messages_lock = &mut data.used_messages.write().await;
    let status_message = messages_lock.entry(server_id).or_default().status;

    match status_message {
        Some(id) => {
            let message_result = http.get_message(channel.id(), id).await;
            match message_result {
                Ok(message) => {
                    let deletion_result = message.delete(http.clone()).await;
                    if let Err(err) = deletion_result {
                        log::error!(
                            "[server {}] Failed to delete old status message: {}",
                            server_id,
                            err
                        );
                        return;
                    } else {
                        log::info!("[server {}] Deleted old status message", server_id);
                    }
                    let send_result = channel
                        .id()
                        .send_message(http, CreateMessage::new().embed(embed.clone()))
                        .await;
                    match send_result {
                        Ok(message) => {
                            messages_lock
                                .insert(server_id, ServerUsedMessages::new(Some(message.id)));
                            log::info!(
                                "[server {}] Sent new status message with id {}",
                                server_id,
                                message.id
                            );
                        }
                        Err(err) => {
                            log::error!(
                                "[server {}] Failed to send new status message: {}",
                                server_id,
                                err
                            );
                        }
                    }
                }
                Err(err) => {
                    log::warn!(
                        "[server {}] Failed to fetch status message because of: {}. Creating new one...",
                        server_id,
                        err
                    );
                    let send_result = channel
                        .id()
                        .send_message(http, CreateMessage::new().embed(embed.clone()))
                        .await;
                    match send_result {
                        Ok(message) => {
                            messages_lock
                                .insert(server_id, ServerUsedMessages::new(Some(message.id)));
                            log::info!(
                                "[server {}] Sent new status message with id {}",
                                server_id,
                                message.id
                            );
                        }
                        Err(err) => {
                            log::error!(
                                "[server {}] Failed to send new status message: {}",
                                server_id,
                                err
                            );
                        }
                    }
                }
            }
        }
        None => {
            log::info!("No status message detected. Creating new one...",);
            let send_result = channel
                .id()
                .send_message(http, CreateMessage::new().embed(embed.clone()))
                .await;
            match send_result {
                Ok(message) => {
                    messages_lock.insert(server_id, ServerUsedMessages::new(Some(message.id)));
                    log::info!(
                        "[server {}] Sent new status message with id {}",
                        server_id,
                        message.id
                    );
                }
                Err(err) => {
                    log::error!(
                        "[server {}] Failed to send new status message: {}",
                        server_id,
                        err
                    );
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
                .colour((21, 250, 59))
                .title(format!("{} is online!", resource_name));
        }
        ResourceStatus::Down => {
            new_embed = new_embed
                .colour((220, 23, 30))
                .title(format!("{} is offline!", resource_name));
        }
        ResourceStatus::Unknown => {
            new_embed = new_embed
                .colour((215, 187, 10))
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

fn replace_templates(message: &str, resource_name: &str, role_id: &Option<RoleId>) -> String {
    let role_ping = match role_id {
        Some(id) => {
            format!("<@&{}>", id)
        }
        None => ROLE_FALLBACK_STRING.to_string(),
    };
    message
        .replace(TEMPLATE_RESOURCE_NAME, resource_name)
        .replace(TEMPLATE_ROLE_PING, role_ping.as_str())
}

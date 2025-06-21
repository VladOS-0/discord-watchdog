mod config;
mod debug;
mod server;

use std::collections::btree_map::{Entry, OccupiedEntry, VacantEntry};

use poise::{
    CreateReply, send_reply,
    serenity_prelude::{CreateAttachment, CreateEmbed, GuildId},
};
use tokio::sync::RwLockWriteGuard;

use crate::{Config, Context, Data, Error, ServerConfig};

pub fn get_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        config::config(),
        debug::info(),
        debug::debug(),
        server::server(),
    ]
}

async fn master_check(ctx: Context<'_>) -> bool {
    match ctx.guild_id() {
        Some(id) => {
            let master = ctx.data().config.read().await.master_server;
            Some(id) == master
        }
        None => false,
    }
}

async fn simple_reply_text(ctx: Context<'_>, ephemeral: bool, text: String) {
    let server_string = match ctx.guild() {
        Some(server) => {
            format!("{} ({})", server.name, server.id)
        }
        None => "UNKNOWN".to_string(),
    };
    let reply_result = send_reply(
        ctx,
        CreateReply::default().ephemeral(ephemeral).content(text),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!(
            "server {:?}] Failed to send reply to a slash command: {}",
            server_string,
            err
        )
    }
}

async fn simple_reply_embed(ctx: Context<'_>, ephemeral: bool, embed: CreateEmbed) {
    let server_string = match ctx.guild() {
        Some(server) => {
            format!("{} ({})", server.name, server.id)
        }
        None => "UNKNOWN".to_string(),
    };
    let reply_result = send_reply(
        ctx,
        CreateReply::default().ephemeral(ephemeral).embed(embed),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!(
            "server {:?}] Failed to send reply to a slash command: {}",
            server_string,
            err
        )
    }
}

async fn simple_reply_attachment(ctx: Context<'_>, ephemeral: bool, attachment: CreateAttachment) {
    let server_string = match ctx.guild() {
        Some(server) => {
            format!("{} ({})", server.name, server.id)
        }
        None => "UNKNOWN".to_string(),
    };
    let reply_result = send_reply(
        ctx,
        CreateReply::default()
            .ephemeral(ephemeral)
            .attachment(attachment),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!(
            "server {:?}] Failed to send reply to a slash command: {}",
            server_string,
            err
        )
    }
}

fn get_server_config_entry<'a>(
    id: Option<GuildId>,
    config_lock: &'a mut RwLockWriteGuard<'_, Config>,
) -> anyhow::Result<OccupiedEntry<'a, GuildId, ServerConfig>> {
    match id {
        Some(id) => {
            let server_config_entry = config_lock.server_configs.entry(id);
            match server_config_entry {
                Entry::Vacant(_) => Err(anyhow::Error::msg(
                    "Your server is not registered yet! Use */server register*!",
                )),
                Entry::Occupied(entry) => Ok(entry),
            }
        }
        None => Err(anyhow::Error::msg(
            "You need to be within a server to execute this command!",
        )),
    }
}

fn get_server_config_vacant_entry<'a>(
    id: Option<GuildId>,
    config_lock: &'a mut RwLockWriteGuard<'_, Config>,
) -> anyhow::Result<VacantEntry<'a, GuildId, ServerConfig>> {
    match id {
        Some(id) => {
            let server_config_entry = config_lock.server_configs.entry(id);
            match server_config_entry {
                Entry::Vacant(entry) => Ok(entry),
                Entry::Occupied(_) => Err(anyhow::Error::msg("Your server is already registered!")),
            }
        }
        None => Err(anyhow::Error::msg(
            "You need to be within a server to execute this command!",
        )),
    }
}

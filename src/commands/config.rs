use std::time::Duration;

use poise::serenity_prelude::{Channel, Role};

use super::master_check;
use crate::{
    Config, Context, DEFAULT_CONFIG_PATH, Error,
    commands::{get_server_config_entry, simple_reply_text},
    ping::resolve_ip,
    save_data,
};

#[derive(poise::ChoiceParameter, Debug, Clone, Copy)]
enum Status {
    Up,
    Down,
}

/// Base config command. Can not be called directly.
#[poise::command(
    slash_command,
    default_member_permissions = "MANAGE_CHANNELS",
    subcommands(
        "reset", "name", "address", "channel", "role", "interval", "timeout", "attempts", "message"
    )
)]
pub async fn config(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// [M ONLY] ALL SERVERS WILL BE RESET!!! Loads all configuration from Config.toml or hardcoded defaults
#[poise::command(slash_command, guild_cooldown = 60)]
async fn reset(ctx: Context<'_>) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if !master_check(ctx).await {
        simple_reply_text(
            ctx,
            true,
            "This command can only be executed in the Master server (bot's host)".to_string(),
        )
        .await;
        return Ok(());
    }

    log::info!(
        "User {} ({}) reset configuration to defaults",
        ctx.author().name,
        ctx.author().id,
    );
    let loaded_config_result = Config::load_from_file(&DEFAULT_CONFIG_PATH).await;

    // Success
    if let Ok(Some(config)) = loaded_config_result {
        log::info!("Loaded Config");
        *ctx.data().config.write().await = config;

        save_data(ctx.data()).await;

        simple_reply_text(
            ctx,
            true,
            "Reset configuration to Config.toml defaults!".to_string(),
        )
        .await;
        return Ok(());
    }
    // Error while loading Config
    if let Err(err) = loaded_config_result {
        log::error!(
            "Failed to load Config from {}: {}",
            DEFAULT_CONFIG_PATH,
            err
        );
        simple_reply_text(
            ctx,
            true,
            format!(
                "Failed to load Config from {}: {}",
                DEFAULT_CONFIG_PATH, err
            ),
        )
        .await;
        return Ok(());
    }
    // No Config
    log::info!("No Config detected. Default values will be used.");
    *ctx.data().config.write().await = Config::default();
    simple_reply_text(
        ctx,
        true,
        "No Config.toml detected - reset configuration to hardcoded defaults!".to_string(),
    )
    .await;

    Ok(())
}

//
//
//
// PING CONFIGURATION
//
//
//

/// [M ONLY] Changes resource address, which is monitored by the bot
#[poise::command(slash_command, guild_cooldown = 20)]
async fn name(
    ctx: Context<'_>,
    #[description = "Name of the resource. It is used in embeds and messages"]
    #[max_length = 25]
    name: String,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if !master_check(ctx).await {
        simple_reply_text(
            ctx,
            true,
            "This command can only be executed in the Master server (bot's host)".to_string(),
        )
        .await;
        return Ok(());
    }

    ctx.data().config.write().await.ping_config.resource_name = name.clone();
    log::info!(
        "User {} ({}) changed resource name to {}",
        ctx.author().name,
        ctx.author().id,
        name
    );

    save_data(ctx.data()).await;

    simple_reply_text(ctx, true, format!("Changed resource name to {}!", name)).await;

    Ok(())
}

/// [M ONLY] Changes resource address, which is monitored by the bot
#[poise::command(slash_command, guild_cooldown = 20)]
async fn address(
    ctx: Context<'_>,
    #[description = "Resource address, which will be pinged"]
    #[max_length = 45]
    #[min_length = 1]
    addr: String,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if !master_check(ctx).await {
        simple_reply_text(
            ctx,
            true,
            "This command can only be executed in the Master server (bot's host)".to_string(),
        )
        .await;
        return Ok(());
    }

    if let Err(err) = resolve_ip(&addr).await {
        simple_reply_text(ctx, true, format!("Failed to resolve your addr: {}", err)).await;
        return Ok(());
    }

    ctx.data().config.write().await.ping_config.resource_addr = addr.clone();
    log::info!(
        "User {} ({}) changed resource address to {}",
        ctx.author().name,
        ctx.author().id,
        addr
    );

    save_data(ctx.data()).await;

    simple_reply_text(ctx, true, format!("Changed resource address to {}!", addr)).await;

    Ok(())
}

/// [M ONLY] Changes interval between ping attempts
#[poise::command(slash_command, guild_cooldown = 20)]
async fn interval(
    ctx: Context<'_>,
    #[description = "New interval between ping attempts in seconds"]
    #[min = 1]
    // 1 day. Hardcoded, yeah.
    #[max = 86_400]
    interval: u64,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if !master_check(ctx).await {
        simple_reply_text(
            ctx,
            true,
            "This command can only be executed in the Master server (bot's host)".to_string(),
        )
        .await;
        return Ok(());
    }

    ctx.data()
        .config
        .write()
        .await
        .ping_config
        .interval_between_attempts = Duration::from_secs(interval);
    log::info!(
        "User {} ({}) changed interval between ping attempts to {} seconds",
        ctx.author().name,
        ctx.author().id,
        interval
    );

    save_data(ctx.data()).await;

    simple_reply_text(
        ctx,
        true,
        format!("Changed interval between ping attempts to {}!", interval),
    )
    .await;

    Ok(())
}

/// [M ONLY] Changes timeout of one ping attempt
#[poise::command(slash_command, guild_cooldown = 20)]
async fn timeout(
    ctx: Context<'_>,
    #[description = "New timeout in seconds"]
    #[min = 1]
    // 1 minute. Hardcoded, yeaaaaaah.
    #[max = 60]
    timeout: u64,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if !master_check(ctx).await {
        simple_reply_text(
            ctx,
            true,
            "This command can only be executed in the Master server (bot's host)".to_string(),
        )
        .await;
        return Ok(());
    }

    ctx.data().config.write().await.ping_config.timeout = Duration::from_secs(timeout);
    log::info!(
        "User {} ({}) changed ping timeout to {} seconds",
        ctx.author().name,
        ctx.author().id,
        timeout
    );

    save_data(ctx.data()).await;

    simple_reply_text(ctx, true, format!("Changed ping timeout to {}!", timeout)).await;

    Ok(())
}

/// [M ONLY] Changes required amount of consecutive attempts, required for resource to change its state
#[poise::command(slash_command, guild_cooldown = 20)]
async fn attempts(
    ctx: Context<'_>,
    #[description = "Resource's status is up && This value is 3 && Ping failed 3 times -> Status changes to down"]
    #[min = 1]
    // 30 attempts. Hardcoded, yeaaaaaaaaaaaaah
    #[max = 30]
    attempts: u8,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if !master_check(ctx).await {
        simple_reply_text(
            ctx,
            true,
            "This command can only be executed in the Master server (bot's host)".to_string(),
        )
        .await;
        return Ok(());
    }

    ctx.data()
        .config
        .write()
        .await
        .ping_config
        .required_attempts_before_notification = attempts;
    log::info!(
        "User {} ({}) changed required attempts to {}",
        ctx.author().name,
        ctx.author().id,
        attempts
    );
    save_data(ctx.data()).await;

    simple_reply_text(
        ctx,
        true,
        format!("Changed required attempts to {}!", attempts),
    )
    .await;

    Ok(())
}

//
//
//
// server CONFIGURATION
//
//
//

/// Changes channel, where bot will send any updates
#[poise::command(slash_command, guild_cooldown = 30)]
async fn channel(
    ctx: Context<'_>,
    #[description = "New channel for updates"] channel: Channel,
) -> Result<(), Error> {
    let server_string = match ctx.guild() {
        Some(server) => {
            format!("{} ({})", server.name, server.id)
        }
        None => "UNKNOWN".to_string(),
    };
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!(
            "[server {}] Failed to defer ephemeral reply: {}",
            server_string,
            err,
        );
    };
    let mut config_lock = ctx.data().config.write().await;
    let mut entry = match get_server_config_entry(ctx.guild_id(), &mut config_lock) {
        Ok(entry) => entry,
        Err(err) => {
            simple_reply_text(ctx, true, err.to_string()).await;
            return Ok(());
        }
    };
    if channel.clone().category().is_some() {
        simple_reply_text(
            ctx,
            true,
            format!(
                "<#{}> is an invalid channel for healthcheck updates!",
                channel.id()
            ),
        )
        .await;
        return Ok(());
    }

    let mut new_server_config = entry.get().clone();
    new_server_config.channel = Some(channel.id());
    entry.insert(new_server_config);

    log::info!(
        "[server {}] User {} ({}) changed channel to {} ({})",
        server_string,
        ctx.author().name,
        ctx.author().id,
        channel,
        channel.id()
    );
    simple_reply_text(
        ctx,
        true,
        format!("Changed channel to <#{}>!", channel.id()),
    )
    .await;

    drop(config_lock);

    save_data(ctx.data()).await;

    Ok(())
}

/// Changes role, which will be pinged by the bot when resource is up
#[poise::command(slash_command, guild_cooldown = 30)]
async fn role(
    ctx: Context<'_>,
    #[description = "New role for notifications"] role: Role,
) -> Result<(), Error> {
    let server_string = match ctx.guild() {
        Some(server) => {
            format!("{} ({})", server.name, server.id)
        }
        None => "UNKNOWN".to_string(),
    };
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!(
            "[server {}] Failed to defer ephemeral reply: {}",
            server_string,
            err,
        );
    };
    let mut config_lock = ctx.data().config.write().await;
    let mut entry = match get_server_config_entry(ctx.guild_id(), &mut config_lock) {
        Ok(entry) => entry,
        Err(err) => {
            simple_reply_text(ctx, true, err.to_string()).await;
            return Ok(());
        }
    };
    if !role.mentionable {
        simple_reply_text(ctx, true, format!("{} can not be mentioned!", role.name)).await;
        return Ok(());
    }

    let mut new_server_config = entry.get().clone();
    new_server_config.role_to_notify = Some(role.id);
    entry.insert(new_server_config);

    log::info!(
        "[server {}] User {} ({}) changed mentionable role to {} ({})",
        server_string,
        ctx.author().name,
        ctx.author().id,
        role.name,
        role.id
    );
    simple_reply_text(
        ctx,
        true,
        format!("Changed mentionable role to <@&{}>!", role.id),
    )
    .await;

    drop(config_lock);

    save_data(ctx.data()).await;

    Ok(())
}

/// Changes required amount of consecutive attempts, after which resource will change its state
#[poise::command(slash_command, guild_cooldown = 30)]
async fn message(
    ctx: Context<'_>,
    #[description = "Whether your message will be sent on Up or Down resource's status change"]
    status: Status,
    #[description = "Message, which will be sent. Remember about %%RESOURCE%% and %%ROLE%% template variables!"]
    #[max_length = 300]
    #[min_length = 1]
    message: String,
) -> Result<(), Error> {
    let server_string = match ctx.guild() {
        Some(server) => {
            format!("{} ({})", server.name, server.id)
        }
        None => "UNKNOWN".to_string(),
    };
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!(
            "[server {}] Failed to defer ephemeral reply: {}",
            server_string,
            err,
        );
    };
    let mut config_lock = ctx.data().config.write().await;
    let mut entry = match get_server_config_entry(ctx.guild_id(), &mut config_lock) {
        Ok(entry) => entry,
        Err(err) => {
            simple_reply_text(ctx, true, err.to_string()).await;
            return Ok(());
        }
    };

    let mut new_server_config = entry.get().clone();

    match status {
        Status::Up => new_server_config.up_message = message.clone(),
        Status::Down => new_server_config.down_message = message.clone(),
    }
    entry.insert(new_server_config);

    log::info!(
        "[server {}] User {} ({}) changed {:?} message to {}",
        server_string,
        ctx.author().name,
        ctx.author().id,
        status,
        message
    );
    simple_reply_text(
        ctx,
        true,
        format!("Changed {:?} message to {}!", status, message),
    )
    .await;

    drop(config_lock);

    save_data(ctx.data()).await;

    Ok(())
}

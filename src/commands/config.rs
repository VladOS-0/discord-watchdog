use std::time::Duration;

use poise::{
    CreateReply, send_reply,
    serenity_prelude::{Channel, Role},
};

use crate::{Config, Context, DEFAULT_CONFIG_PATH, Error, ping::resolve_ip, save_data};

/// Base config command. Can not be called directly.
#[poise::command(
    slash_command,
    default_member_permissions = "MANAGE_CHANNELS",
    subcommands(
        "reset", "name", "address", "channel", "role", "interval", "timeout", "attempts"
    )
)]
pub async fn config(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Loads all configuration from Config.toml or hardcoded defaults
#[poise::command(slash_command, user_cooldown = 12)]
async fn reset(ctx: Context<'_>) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    log::info!(
        "User {} ({}) reset configuration to defaults",
        ctx.author().name,
        ctx.author().id,
    );
    let loaded_config_result = Config::load_from_file(&DEFAULT_CONFIG_PATH).await;
    if let Ok(Some(config)) = loaded_config_result {
        log::info!("Loaded Config");
        *ctx.data().config.write().await = config;
        save_data(ctx.data()).await;
        let reply_result = send_reply(
            ctx,
            CreateReply::default()
                .ephemeral(true)
                .content("Reset configuration to Config.toml defaults!".to_string()),
        )
        .await;
        if let Err(err) = reply_result {
            log::error!("Failed to send reply to a slash command: {}", err)
        }
    } else if let Err(err) = loaded_config_result {
        log::error!("Failed to load Config: {}", err);
        let reply_result = send_reply(
            ctx,
            CreateReply::default()
                .ephemeral(true)
                .content(format!("Failed to load Config.toml: {}", err)),
        )
        .await;
        if let Err(err) = reply_result {
            log::error!("Failed to send reply to a slash command: {}", err)
        }
    } else {
        log::info!("No Config detected. Default values will be used.");
        *ctx.data().config.write().await = Config::default();
        let reply_result = send_reply(
            ctx,
            CreateReply::default().ephemeral(true).content(
                "No Config.toml detected - reset configuration to hardcoded defaults!".to_string(),
            ),
        )
        .await;
        if let Err(err) = reply_result {
            log::error!("Failed to send reply to a slash command: {}", err)
        }
    }

    Ok(())
}

/// Changes resource address, which is monitored by the bot
#[poise::command(slash_command, user_cooldown = 12)]
async fn name(
    ctx: Context<'_>,
    #[description = "Name of the resource. It is used in embeds and messages"]
    #[max_length = 25]
    name: String,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    ctx.data().config.write().await.resource_name = name.clone();
    log::info!(
        "User {} ({}) changed resource name to {}",
        ctx.author().name,
        ctx.author().id,
        name
    );
    save_data(ctx.data()).await;
    let reply_result = send_reply(
        ctx,
        CreateReply::default()
            .ephemeral(true)
            .content(format!("Changed resource name to {}!", name)),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!("Failed to send reply to a slash command: {}", err)
    }

    Ok(())
}

/// Changes resource address, which is monitored by the bot
#[poise::command(slash_command, user_cooldown = 12)]
async fn address(
    ctx: Context<'_>,
    #[description = "Resource address, which will be pinged"]
    #[max_length = 45]
    addr: String,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if let Err(err) = resolve_ip(&addr).await {
        let reply_result = send_reply(
            ctx,
            CreateReply::default()
                .ephemeral(true)
                .content(format!("Failed to resolve your addr: {}", err)),
        )
        .await;
        if let Err(err) = reply_result {
            log::error!("Failed to send reply to a slash command: {}", err)
        }
    } else {
        ctx.data().config.write().await.resource_addr = addr.clone();
        log::info!(
            "User {} ({}) changed resource address to {}",
            ctx.author().name,
            ctx.author().id,
            addr
        );
        save_data(ctx.data()).await;
        let reply_result = send_reply(
            ctx,
            CreateReply::default()
                .ephemeral(true)
                .content(format!("Changed resource address to {}!", addr)),
        )
        .await;
        if let Err(err) = reply_result {
            log::error!("Failed to send reply to a slash command: {}", err)
        }
    }
    Ok(())
}

/// Changes channel, where bot will send any updates
#[poise::command(slash_command, user_cooldown = 12)]
async fn channel(
    ctx: Context<'_>,
    #[description = "New channel for updates"] channel: Channel,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if channel.clone().category().is_some() {
        let reply_result = send_reply(
            ctx,
            CreateReply::default().ephemeral(true).content(format!(
                "<#{}> is an invalid channel for healthcheck updates!",
                channel.id()
            )),
        )
        .await;
        if let Err(err) = reply_result {
            log::error!("Failed to send reply to a slash command: {}", err)
        }
        return Ok(());
    }
    ctx.data().config.write().await.channel = Some(channel.id());
    ctx.data().used_messages.write().await.status = None;
    log::info!(
        "User {} ({}) changed channel to {} ({})",
        ctx.author().name,
        ctx.author().id,
        channel,
        channel.id()
    );
    save_data(ctx.data()).await;
    let reply_result = send_reply(
        ctx,
        CreateReply::default()
            .ephemeral(true)
            .content(format!("Changed channel to <#{}>!", channel.id())),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!("Failed to send reply to a slash command: {}", err)
    }

    Ok(())
}

/// Changes role, which will be pinged by the bot when resource is up
#[poise::command(slash_command, user_cooldown = 12)]
async fn role(
    ctx: Context<'_>,
    #[description = "New role for notifications"] role: Role,
) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    if !role.mentionable {
        let reply_result = send_reply(
            ctx,
            CreateReply::default()
                .ephemeral(true)
                .content(format!("{} can not be mentioned!", role.name)),
        )
        .await;
        if let Err(err) = reply_result {
            log::error!("Failed to send reply to a slash command: {}", err)
        }
        return Ok(());
    }
    ctx.data().config.write().await.role_to_notify = Some(role.id);
    log::info!(
        "User {} ({}) changed mentionable role to {} ({})",
        ctx.author().name,
        ctx.author().id,
        role.name,
        role.id
    );
    save_data(ctx.data()).await;
    let reply_result = send_reply(
        ctx,
        CreateReply::default()
            .ephemeral(true)
            .content(format!("Changed mentionable role to <@&{}>!", role.id)),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!("Failed to send reply to a slash command: {}", err)
    }

    Ok(())
}

/// Changes interval between ping attempts
#[poise::command(slash_command, user_cooldown = 12)]
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
    ctx.data().config.write().await.interval_between_attempts = Duration::from_secs(interval);
    log::info!(
        "User {} ({}) changed interval between ping attempts to {} seconds",
        ctx.author().name,
        ctx.author().id,
        interval
    );
    save_data(ctx.data()).await;
    let reply_result = send_reply(
        ctx,
        CreateReply::default().ephemeral(true).content(format!(
            "Changed interval between ping attempts to {}!",
            interval
        )),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!("Failed to send reply to a slash command: {}", err)
    }

    Ok(())
}

/// Changes timeout of one ping attempt
#[poise::command(slash_command, user_cooldown = 12)]
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
    ctx.data().config.write().await.timeout = Duration::from_secs(timeout);
    log::info!(
        "User {} ({}) changed ping timeout to {} seconds",
        ctx.author().name,
        ctx.author().id,
        timeout
    );
    save_data(ctx.data()).await;
    let reply_result = send_reply(
        ctx,
        CreateReply::default()
            .ephemeral(true)
            .content(format!("Changed ping timeout to {}!", timeout)),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!("Failed to send reply to a slash command: {}", err)
    }

    Ok(())
}

/// Changes required amount of consecutive attempts, after which resource will change its state
#[poise::command(slash_command, user_cooldown = 12)]
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
    ctx.data()
        .config
        .write()
        .await
        .required_attempts_before_notification = attempts;
    log::info!(
        "User {} ({}) changed required attempts to {}",
        ctx.author().name,
        ctx.author().id,
        attempts
    );
    save_data(ctx.data()).await;
    let reply_result = send_reply(
        ctx,
        CreateReply::default()
            .ephemeral(true)
            .content(format!("Changed required attempts to {}!", attempts)),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!("Failed to send reply to a slash command: {}", err)
    }

    Ok(())
}

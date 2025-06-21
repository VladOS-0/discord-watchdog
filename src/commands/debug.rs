use poise::serenity_prelude::{CreateAttachment, CreateEmbed, Timestamp};

use crate::{
    Context, DEFAULT_LOG_PATH, DEFAULT_REPOSITORY, DEFAULT_SAVEDATA_PATH, Error, THIS_RUN_START,
    commands::{master_check, simple_reply_attachment, simple_reply_embed, simple_reply_text},
};

/// Displays information about the bot
#[poise::command(slash_command, user_cooldown = 10)]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    if let Err(err) = ctx.defer().await {
        log::error!("Failed to defer reply: {}", err);
    };
    simple_reply_embed(
        ctx,
        false,
        CreateEmbed::new()
            .title(format!(
                "**Discord Watchdog v{}**",
                env!("CARGO_PKG_VERSION")
            ))
            .colour((45, 114, 178))
            .fields(vec![
                (
                    "Running since",
                    format!(
                        "<t:{}:R>",
                        THIS_RUN_START
                            .get()
                            .unwrap_or(&Timestamp::now())
                            .unix_timestamp()
                    ),
                    false,
                ),
                (
                    "Repository",
                    format!("[Github]({})", DEFAULT_REPOSITORY),
                    false,
                ),
            ]),
    )
    .await;
    Ok(())
}

/// Base debug command. Can not be called directly.
#[poise::command(
    slash_command,
    default_member_permissions = "MANAGE_CHANNELS",
    subcommands("logs", "data")
)]
pub async fn debug(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// [M ONLY] Sends a log file
#[poise::command(slash_command, guild_cooldown = 40)]
async fn logs(ctx: Context<'_>) -> Result<(), Error> {
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

    let attachment_result = CreateAttachment::path(DEFAULT_LOG_PATH).await;

    match attachment_result {
        Ok(attachment) => {
            log::info!(
                "User {} ({}) requested {}",
                ctx.author().name,
                ctx.author().id,
                DEFAULT_LOG_PATH
            );
            simple_reply_attachment(ctx, true, attachment).await;
        }
        Err(err) => {
            log::error!(
                "Failed to retrieve {} on user's demand: {}",
                DEFAULT_LOG_PATH,
                err
            );
            simple_reply_text(
                ctx,
                true,
                format!("Failed to retrieve {}: {}", DEFAULT_LOG_PATH, err),
            )
            .await;
        }
    }
    Ok(())
}

/// [M ONLY] Sends a data file
#[poise::command(slash_command, guild_cooldown = 40)]
async fn data(ctx: Context<'_>) -> Result<(), Error> {
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

    let attachment_result = CreateAttachment::path(DEFAULT_SAVEDATA_PATH).await;

    match attachment_result {
        Ok(attachment) => {
            log::info!(
                "User {} ({}) requested {}",
                ctx.author().name,
                ctx.author().id,
                DEFAULT_SAVEDATA_PATH
            );
            simple_reply_attachment(ctx, true, attachment).await;
        }
        Err(err) => {
            log::error!(
                "Failed to retrieve {} on user's demand: {}",
                DEFAULT_SAVEDATA_PATH,
                err
            );
            simple_reply_text(
                ctx,
                true,
                format!("Failed to retrieve {}: {}", DEFAULT_SAVEDATA_PATH, err),
            )
            .await;
        }
    }

    Ok(())
}

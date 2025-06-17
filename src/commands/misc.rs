use poise::{
    CreateReply, send_reply,
    serenity_prelude::{CreateAttachment, CreateEmbed, Timestamp},
};

use crate::{
    Context, DEFAULT_LOG_PATH, DEFAULT_REPOSITORY, DEFAULT_SAVEDATA_PATH, Error, THIS_RUN_START,
};

/// Displays information about the bot
#[poise::command(slash_command, user_cooldown = 5)]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    let reply_result = send_reply(
        ctx,
        CreateReply::default().embed(
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
        ),
    )
    .await;
    if let Err(err) = reply_result {
        log::error!("Failed to send reply to a slash command: {}", err)
    }
    Ok(())
}

/// Base debug command. Can not be called directly.
#[poise::command(
    slash_command,
    default_member_permissions = "MENTION_EVERYONE",
    subcommands("logs", "data")
)]
pub async fn debug(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Sends a log file
#[poise::command(slash_command, user_cooldown = 30, global_cooldown = 10)]
async fn logs(ctx: Context<'_>) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    let attachment_result = CreateAttachment::path(DEFAULT_LOG_PATH).await;
    match attachment_result {
        Ok(attachment) => {
            log::info!(
                "User {} ({}) requested {}",
                ctx.author().name,
                ctx.author().id,
                DEFAULT_LOG_PATH
            );
            let reply_result = send_reply(
                ctx,
                CreateReply::default()
                    .ephemeral(true)
                    .attachment(attachment),
            )
            .await;
            if let Err(err) = reply_result {
                log::error!("Failed to send reply to a slash command: {}", err)
            }
        }
        Err(err) => {
            log::error!(
                "Failed to retrieve {} on user's demand: {}",
                DEFAULT_LOG_PATH,
                err
            );
            let reply_result = send_reply(
                ctx,
                CreateReply::default()
                    .ephemeral(true)
                    .content(format!("Failed to retrieve {}: {}", DEFAULT_LOG_PATH, err)),
            )
            .await;
            if let Err(err) = reply_result {
                log::error!("Failed to send reply to a slash command: {}", err)
            }
        }
    }
    Ok(())
}

/// Sends a data file
#[poise::command(slash_command, user_cooldown = 30, global_cooldown = 10)]
async fn data(ctx: Context<'_>) -> Result<(), Error> {
    if let Err(err) = ctx.defer_ephemeral().await {
        log::error!("Failed to defer ephemeral reply: {}", err);
    };
    let attachment_result = CreateAttachment::path(DEFAULT_SAVEDATA_PATH).await;
    match attachment_result {
        Ok(attachment) => {
            log::info!(
                "User {} ({}) requested {}",
                ctx.author().name,
                ctx.author().id,
                DEFAULT_SAVEDATA_PATH
            );
            let reply_result = send_reply(
                ctx,
                CreateReply::default()
                    .ephemeral(true)
                    .attachment(attachment),
            )
            .await;
            if let Err(err) = reply_result {
                log::error!("Failed to send reply to a slash command: {}", err)
            }
        }
        Err(err) => {
            log::error!(
                "Failed to retrieve {} on user's demand: {}",
                DEFAULT_LOG_PATH,
                err
            );
            let reply_result = send_reply(
                ctx,
                CreateReply::default().ephemeral(true).content(format!(
                    "Failed to retrieve {}: {}",
                    DEFAULT_SAVEDATA_PATH, err
                )),
            )
            .await;
            if let Err(err) = reply_result {
                log::error!("Failed to send reply to a slash command: {}", err)
            }
        }
    }
    Ok(())
}

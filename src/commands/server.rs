use poise::serenity_prelude::{CreateEmbed, GuildId};

use crate::{
    Context, Error, ServerConfig,
    commands::{
        get_server_config_vacant_entry, master_check, simple_reply_embed, simple_reply_text,
    },
    save_data,
};

#[derive(poise::ChoiceParameter, Debug, Clone, Copy)]
enum RemovalOptions {
    All,
    One,
}

/// Base server command. Can not be called directly.
#[poise::command(
    slash_command,
    default_member_permissions = "MANAGE_CHANNELS",
    subcommands("register", "limit", "show", "remove")
)]
pub async fn server(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Adds server to the registry, allowing bot usage
#[poise::command(slash_command, guild_cooldown = 30)]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    let mut server_name = "UNKNOWN".to_string();
    let server_string = match ctx.guild() {
        Some(server) => {
            server_name = server.name.clone();
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
    if config_lock.server_configs.len() >= config_lock.max_servers {
        simple_reply_text(ctx, true, format!(
            "There are already {} servers registered, and bot hoster allows only {} registrations. Contact them to increase this number!",
            config_lock.server_configs.len(), config_lock.max_servers
        )).await;
        return Ok(());
    }
    let entry = match get_server_config_vacant_entry(ctx.guild_id(), &mut config_lock) {
        Ok(entry) => entry,
        Err(err) => {
            simple_reply_text(ctx, true, err.to_string()).await;
            return Ok(());
        }
    };

    entry.insert(ServerConfig::with_name(server_name));

    log::info!(
        "[server {}] server registered by  {} ({})",
        server_string,
        ctx.author().name,
        ctx.author().id,
    );
    simple_reply_text(
        ctx,
        true,
        "server registered! Do not forget to use */config role* and */config channgel*! Also consider using */config message*".to_string()
    )
    .await;

    drop(config_lock);

    save_data(ctx.data()).await;

    Ok(())
}

/// [M ONLY] Changes servers registration limit
#[poise::command(slash_command, guild_cooldown = 20)]
async fn limit(
    ctx: Context<'_>,
    #[description = "Maximum amount of registrated servers"]
    #[min = 1]
    #[max = 100]
    limit: usize,
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

    ctx.data().config.write().await.max_servers = limit;
    log::info!(
        "User {} ({}) changed servers limit to {}",
        ctx.author().name,
        ctx.author().id,
        limit
    );

    save_data(ctx.data()).await;

    simple_reply_text(ctx, true, format!("Changed servers limit to {}!", limit)).await;

    Ok(())
}

/// [M ONLY] Shows all registraded servers
#[poise::command(slash_command, guild_cooldown = 20)]
async fn show(ctx: Context<'_>) -> Result<(), Error> {
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

    let mut result_embed = CreateEmbed::new()
        .colour((45, 114, 178))
        .title("Registered servers");
    for (server_id, server_config) in &ctx.data().config.read().await.server_configs {
        result_embed = result_embed.field(server_config.name.clone(), server_id.to_string(), false);
    }
    log::info!(
        "User {} ({}) checked servers list",
        ctx.author().name,
        ctx.author().id,
    );

    simple_reply_embed(ctx, true, result_embed).await;

    save_data(ctx.data()).await;

    Ok(())
}

/// [M ONLY] Removes one or all registered servers except for master server
#[poise::command(slash_command, guild_cooldown = 20)]
async fn remove(
    ctx: Context<'_>,
    #[description = "Remove all servers or one with the provided ID? (can be checked in /server show)"]
    options: RemovalOptions,
    #[description = "ID of the server, which will be removed"] id: Option<GuildId>,
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
    let master_server_id = ctx.guild_id().expect(
        "Master check is broken: failed to get master server id, while check was successful",
    );
    match options {
        RemovalOptions::All => {
            log::info!(
                "User {} ({}) unregistered all servers.",
                ctx.author().name,
                ctx.author().id,
            );
            let config_lock = ctx.data().config.read().await;
            let master_config = config_lock.server_configs.get(&master_server_id);
            match master_config {
                Some(config) => {
                    let master_backup = config.clone();
                    drop(config_lock);

                    let mut config_lock = ctx.data().config.write().await;
                    config_lock.server_configs.clear();
                    config_lock
                        .server_configs
                        .insert(master_server_id, master_backup);
                    drop(config_lock);

                    simple_reply_text(ctx, true, "All servers were removed!".to_string()).await;
                }
                None => {
                    log::warn!("No master config was detected");
                    drop(config_lock);

                    ctx.data().config.write().await.server_configs.clear();

                    simple_reply_text(ctx, true, "All servers were removed!".to_string()).await;
                }
            }
        }
        RemovalOptions::One => {
            if let Some(id) = id {
                if id == master_server_id {
                    simple_reply_text(ctx, true, "You can not remove master server!".to_string())
                        .await;
                    return Ok(());
                }
                let removed_entry = ctx
                    .data()
                    .config
                    .write()
                    .await
                    .server_configs
                    .remove_entry(&id);
                if let Some((removed_id, removed_config)) = removed_entry {
                    log::info!(
                        "User {} ({}) unregistered server {} ({}).",
                        ctx.author().name,
                        ctx.author().id,
                        removed_config.name,
                        removed_id
                    );
                    simple_reply_text(
                        ctx,
                        true,
                        format!(
                            "server {} ({}) was removed!",
                            removed_config.name, removed_id
                        ),
                    )
                    .await;
                }
            } else {
                simple_reply_text(
                    ctx,
                    true,
                    "You need to provide server ID, if you want to remove one!".to_string(),
                )
                .await;
                return Ok(());
            }
        }
    }

    save_data(ctx.data()).await;

    Ok(())
}

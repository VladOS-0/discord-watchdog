use std::{process::exit, sync::Arc, time::SystemTime};

use dotenv::dotenv;
use poise::serenity_prelude::{self as serenity, GatewayIntents, Http, Timestamp};

use discord_watchdog::{
    Config, DEFAULT_CONFIG_PATH, DEFAULT_LOG_PATH, DEFAULT_SAVEDATA_PATH, Data, SavedData,
    THIS_RUN_START, commands::get_commands, ping::ping_task,
};

#[tokio::main]
async fn main() {
    THIS_RUN_START.set(Timestamp::now()).unwrap_or_else(|err| {
        eprintln!("Failed to set THIS_RUN_START: {}. Execution halted.", err);
        // Things have gone really bad and we can't even check INTERACTIVE at this point, let's just assume it is 1
        println!("Press any button to exit...");
        std::io::stdin().read_line(&mut String::new()).unwrap();
        exit(1)
    });
    dotenv()
        .inspect(|dotenv_file| {
            println!("Loaded .env file from {}", dotenv_file.display());
        })
        .map_err(|err| {
            eprintln!("Failed to load .env file: {}", err);
            err
        })
        .ok();
    let interactive = std::env::var("INTERACTIVE")
        .unwrap_or("1".to_string())
        .parse::<u8>()
        .unwrap_or(1)
        .eq(&1);
    setup_logger().unwrap_or_else(|err| {
        eprintln!("Failed to set up logger: {}. Execution halted.", err);
        if interactive {
            println!("Press any button to exit...");
            std::io::stdin().read_line(&mut String::new()).unwrap();
        }
        exit(1)
    });

    log::info!("Discord Watchdog v{}", env!("CARGO_PKG_VERSION"));

    let context = init_data().await;

    let context_ping_task = context.clone();
    let token = std::env::var("DISCORD_TOKEN").unwrap_or_else(|err| {
        log::error!("No Discord token detected: {}. Execution halted.", err);
        if interactive {
            println!("Press any button to exit...");
            std::io::stdin().read_line(&mut String::new()).unwrap();
        }
        exit(1)
    });
    let http = Arc::new(Http::new(&token));
    let intents = serenity::GatewayIntents::non_privileged().union(GatewayIntents::GUILD_MESSAGES);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: get_commands(),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(context)
            })
        })
        .build();
    let client_result = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    match client_result {
        Ok(mut client) => {
            log::info!("Discord client started");
            // Actual main loop divided into 2 green threads: receiving users' commands and checking service health.
            tokio::select! {
                client_exec_result = client.start() => {
                    log::warn!("Discord client exited with: {:?}. Execution halted.", client_exec_result);
                    if interactive {
                        println!("Press any button to exit...");
                        std::io::stdin().read_line(&mut String::new()).unwrap();
                    }
                    exit(1)

                }
                ping_task_result = ping_task(context_ping_task, http.clone()) => {
                    log::warn!("Ping task exited with {:?}. Execution halted.", ping_task_result);
                    if interactive {
                        println!("Press any button to exit...");
                        std::io::stdin().read_line(&mut String::new()).unwrap();
                    }
                    exit(1)
                }
            };
        }
        Err(err) => {
            log::error!("Failed to build Discord client: {}. Execution halted.", err);
            if interactive {
                println!("Press any button to exit...");
                std::io::stdin().read_line(&mut String::new()).unwrap();
            }
            exit(1)
        }
    }
}

fn setup_logger() -> Result<(), fern::InitError> {
    let tracing = std::env::var("TRACING")
        .unwrap_or("0".to_string())
        .parse::<u8>()
        .unwrap_or(0)
        .ne(&0);
    let mut dispatch = fern::Dispatch::new().format(|out, message, record| {
        out.finish(format_args!(
            "[{} {} {}] {}",
            humantime::format_rfc3339_seconds(SystemTime::now()),
            record.level(),
            record.target(),
            message
        ))
    });
    if tracing {
        dispatch = dispatch.level_for("discord_watchdog", log::LevelFilter::Trace);
    } else {
        dispatch = dispatch.level_for("discord_watchdog", log::LevelFilter::Debug);
    }
    dispatch
        .level(log::LevelFilter::Error)
        .chain(std::io::stdout())
        .chain(fern::log_file(DEFAULT_LOG_PATH)?)
        .apply()?;
    Ok(())
}

async fn init_data() -> Data {
    // Create default context
    let data: Data = Data::default();

    // Load SaveData if any
    let saved_data_result = SavedData::load_from_file(&DEFAULT_SAVEDATA_PATH).await;

    match saved_data_result {
        Ok(saved_data_option) => match saved_data_option {
            Some(saved_data) => {
                saved_data.load_into(&data).await;
                log::info!("Loaded SavedData");
            }
            None => {
                log::info!("No SaveData detected. Initializing...");
                let mut new_saved_data = SavedData::default();
                let loaded_config_result = Config::load_from_file(&DEFAULT_CONFIG_PATH).await;
                if let Ok(Some(config)) = loaded_config_result {
                    log::info!("Loaded Config");
                    new_saved_data.config = config
                } else if let Err(err) = loaded_config_result {
                    log::error!("Failed to load Config: {}", err);
                } else {
                    log::info!("No Config detected. Default values will be used.")
                }
                new_saved_data.load_into(&data).await;
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
        },
        Err(err) => {
            log::error!("Failed to load SaveData: {}", err);
        }
    }

    data
}

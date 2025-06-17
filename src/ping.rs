use std::{net::IpAddr, sync::Arc, time::Duration};

use anyhow::Error;
use poise::serenity_prelude::Http;
use tokio::{task, time};

use crate::{DEFAULT_INTERVAL_BETWEEN_ATTEMPTS_SECS, Data, ResourceStatus, status::update_status};

pub async fn ping_task(data: Data, http: Arc<Http>) -> Result<(), task::JoinError> {
    let task = task::spawn(async move {
        let mut interval =
            time::interval(Duration::from_secs(DEFAULT_INTERVAL_BETWEEN_ATTEMPTS_SECS));

        loop {
            interval.tick().await;
            let config_lock = data.config.read().await;
            let interval_duration = config_lock.interval_between_attempts;
            let timeout = config_lock.timeout;
            let addr = &config_lock.resource_addr;
            interval = time::interval(interval_duration);
            interval.tick().await;

            let response = healthcheck(addr, timeout).await;
            drop(config_lock);

            match response {
                Ok(success) => {
                    if success {
                        update_status(ResourceStatus::Up, data.clone(), http.clone()).await;
                    } else {
                        update_status(ResourceStatus::Down, data.clone(), http.clone()).await;
                    }
                }
                Err(err) => {
                    log::error!("Failed to healthcheck: {}", err);
                    update_status(ResourceStatus::Unknown, data.clone(), http.clone()).await;
                }
            }
        }
    });

    task.await
}

pub async fn healthcheck(addr: &str, timeout: Duration) -> anyhow::Result<bool> {
    let mut config_builder = surge_ping::Config::builder();
    let ip = resolve_ip(addr).await?;
    if ip.is_ipv6() {
        config_builder = config_builder.kind(surge_ping::ICMP::V6);
    }
    let config = config_builder.build();
    let client = surge_ping::Client::new(&config)?;
    let mut pinger = client.pinger(ip, surge_ping::PingIdentifier(111)).await;
    pinger.timeout(timeout);

    match pinger.ping(surge_ping::PingSequence(0), &[0]).await {
        Ok((_, rtt)) => {
            log::trace!("Pinging {} resulted in success in {:0.2?}", addr, rtt);
            Ok(true)
        }
        Err(err) => match err {
            surge_ping::SurgeError::Timeout { seq } => {
                log::trace!("Pinging {addr} with sequence {seq} resulted in timeout.");
                Ok(false)
            }
            _ => Err(Error::msg(format!("Failed to ping {}: {}", addr, err))),
        },
    }
}

pub async fn resolve_ip(addr: &str) -> anyhow::Result<IpAddr> {
    let ip = tokio::net::lookup_host(format!("{}:0", addr))
        .await?
        .next()
        .map(|val| val.ip())
        .ok_or(Error::msg(format!(
            "Failed to resolve DNS for domain {addr}: No IP associated with it"
        )))?;
    Ok(ip)
}

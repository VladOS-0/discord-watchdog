use std::{net::IpAddr, process, sync::Arc, time::Duration};

use anyhow::Error;
use poise::serenity_prelude::Http;
use tokio::{task, time};

use crate::{DEFAULT_INTERVAL_BETWEEN_ATTEMPTS_SECS, Data, ResourceStatus, status::update_status};

const DEFAULT_ICMP_PAYLOAD: [u8; 1] = [1];

pub async fn ping_task(data: Data, http: Arc<Http>) -> Result<(), task::JoinError> {
    let task = task::spawn(async move {
        let mut interval =
            time::interval(Duration::from_secs(DEFAULT_INTERVAL_BETWEEN_ATTEMPTS_SECS));
        let mut icmp_sequence: u16 = 0;
        let icmp_id: u16 = process::id() as u16;

        loop {
            interval.tick().await;
            icmp_sequence += 1;

            let config_lock = data.config.read().await;
            let interval_duration = config_lock.ping_config.interval_between_attempts;
            let timeout = config_lock.ping_config.timeout;
            let addr = config_lock.ping_config.resource_addr.clone();
            drop(config_lock);

            interval = time::interval(interval_duration);
            interval.tick().await;

            let response = healthcheck(&addr, timeout, icmp_sequence, icmp_id).await;

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

pub async fn healthcheck(
    addr: &str,
    timeout: Duration,
    icmp_sequence: u16,
    icmp_id: u16,
) -> anyhow::Result<bool> {
    let mut config_builder = surge_ping::Config::builder();
    let ip = resolve_ip(addr).await?;
    if ip.is_ipv6() {
        config_builder = config_builder.kind(surge_ping::ICMP::V6);
    }
    let config = config_builder.build();
    let client = surge_ping::Client::new(&config)?;
    let mut pinger = client.pinger(ip, surge_ping::PingIdentifier(icmp_id)).await;
    pinger.timeout(timeout);

    match pinger
        .ping(
            surge_ping::PingSequence(icmp_sequence),
            &DEFAULT_ICMP_PAYLOAD,
        )
        .await
    {
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
#[cfg(test)]
mod tests {
    use std::{process, time::Duration};

    use crate::{DEFAULT_TIMEOUT_SECS, ping::healthcheck};

    // let's just hope that google will not go down while we are testing
    const SUCCESSFUL_HEALTHCHECK_ADDR: &str = "google.com";
    const TIMEOUT_HEALTHCHECK_ADDR: &str = "1123";
    const FAILING_HEALTHCHECK_ADDR: &str = "fwrgrwetf3";

    #[tokio::test]
    #[cfg_attr(feature = "ci", ignore)]
    async fn healthcheck_success() {
        let icmp_sequence: u16 = 0;
        let icmp_id: u16 = process::id() as u16;

        let healthcheck_result = healthcheck(
            SUCCESSFUL_HEALTHCHECK_ADDR,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            icmp_sequence,
            icmp_id,
        )
        .await;

        assert!(
            &healthcheck_result.as_ref().is_ok_and(|ok| *ok),
            "Healthchecking {} failed: {:?}",
            SUCCESSFUL_HEALTHCHECK_ADDR,
            healthcheck_result
        );
    }

    #[tokio::test]
    #[cfg_attr(feature = "ci", ignore)]
    async fn healthcheck_timeout() {
        let icmp_sequence: u16 = 0;
        let icmp_id: u16 = process::id() as u16;

        let healthcheck_result = healthcheck(
            TIMEOUT_HEALTHCHECK_ADDR,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            icmp_sequence,
            icmp_id,
        )
        .await;

        assert!(
            &healthcheck_result.as_ref().is_ok_and(|ok| !*ok),
            "Healthchecking address {} did not result in a timeout: {:?}",
            TIMEOUT_HEALTHCHECK_ADDR,
            healthcheck_result
        );
    }

    #[tokio::test]
    #[cfg_attr(feature = "ci", ignore)]
    async fn healthcheck_error() {
        let icmp_sequence: u16 = 0;
        let icmp_id: u16 = process::id() as u16;

        let healthcheck_result = healthcheck(
            FAILING_HEALTHCHECK_ADDR,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            icmp_sequence,
            icmp_id,
        )
        .await;

        assert!(
            &healthcheck_result.as_ref().is_err(),
            "Healthchecking non-existing address {} succeeded: {:?}",
            FAILING_HEALTHCHECK_ADDR,
            healthcheck_result
        );
    }
}

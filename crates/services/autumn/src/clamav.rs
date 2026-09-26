use std::time::Duration;

use revolt_config::{config, report_internal_error};
use revolt_result::Result;

pub async fn init() {
    let config = config().await;

    if !config.files.clamd_host.is_empty() {
        tracing::info!("Waiting for clamd to be ready...");

        loop {
            let clamd_available =
                match revolt_clamav_client::ping_tcp(config.files.clamd_host.clone()) {
                    Ok(ping_response) => ping_response == b"PONG\0",
                    Err(_) => false,
                };

            if clamd_available {
                tracing::info!("clamd is ready, virus protection enabled!");
                break;
            } else {
                tracing::error!(
                    "Could not ping clamd host at {}, retrying in 10 seconds...",
                    config.files.clamd_host
                );

                std::thread::sleep(Duration::from_secs(10));
            }
        }
    }
}

pub async fn is_malware(buf: &[u8]) -> Result<bool> {
    let config = config().await;
    if config.files.clamd_host.is_empty() {
        Ok(false)
    } else {
        let scan_response = report_internal_error!(revolt_clamav_client::scan_buffer_tcp(
            buf,
            config.files.clamd_host,
            None
        ))?;

        report_internal_error!(revolt_clamav_client::clean(&scan_response)).map(|v| !v)
    }
}

//! Generic BLE discovery and GATT enumeration (btleplug / CoreBluetooth).

use anyhow::{Context, Result, anyhow};
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use futures::StreamExt;
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::time::{Duration, Instant};

pub const NAME_PREFIX: &str = "M511";

pub async fn adapter() -> Result<Adapter> {
    let manager = Manager::new().await?;
    manager.adapters().await?.into_iter().next().ok_or_else(|| anyhow!("no Bluetooth adapter"))
}

/// Scan until a peripheral whose local name starts with `prefix` shows up, or `timeout` passes.
pub async fn find(adapter: &Adapter, prefix: &str, timeout: Duration) -> Result<Peripheral> {
    adapter.start_scan(ScanFilter::default()).await.context("start scan")?;
    let deadline = Instant::now() + timeout;
    // CoreBluetooth keeps peripherals from earlier scans, so only trust a live advertisement event.
    let mut events = adapter.events().await?;
    let mut found = None;
    while found.is_none() {
        let Ok(Some(ev)) = tokio::time::timeout_at(deadline.into(), events.next()).await else { break };
        let id = match ev {
            CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => id,
            CentralEvent::ManufacturerDataAdvertisement { id, .. } | CentralEvent::ServicesAdvertisement { id, .. } => id,
            _ => continue,
        };
        if let Ok(p) = adapter.peripheral(&id).await
            && let Ok(Some(props)) = p.properties().await
            && props.local_name.as_deref().is_some_and(|n| n.starts_with(prefix))
        {
            found = Some(p);
        }
    }
    adapter.stop_scan().await.ok();
    found.ok_or_else(|| anyhow!("no {prefix}* advertising within {timeout:?} (press the printer's power button)"))
}

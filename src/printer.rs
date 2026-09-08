//! A connected printer session over btleplug.

use crate::job::{self, JobParams, Raster};
use crate::proto::{self, Properties, Reassembler, Status};
use anyhow::{Context, Result, anyhow, bail};
use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::{Adapter, Peripheral};
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::watch;
use uuid::{Uuid, uuid};

pub const SERVICE: Uuid = uuid!("0000fd1c-0000-1000-8000-00805f9b34fb");
pub const CHAR_JOB: Uuid = uuid!("7d9d9a4d-b530-4d13-8d61-e0ff445add19");
pub const CHAR_REQUEST: Uuid = uuid!("a61ae408-3273-420c-a9db-0669f4f23b69");
pub const CHAR_STATUS: Uuid = uuid!("786af345-1b68-c594-c643-e2867da117e3");

pub struct Printer {
    peripheral: Peripheral,
    job_char: Characteristic,
    request_char: Characteristic,
    props: Arc<Mutex<Properties>>,
    changed: watch::Receiver<u64>,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct JobResult {
    pub job_id: String,
    pub ok: bool,
    pub status: String,
    pub bytes: usize,
    pub fragments: usize,
}

impl Printer {
    pub async fn connect(adapter: &Adapter, scan_timeout: Duration) -> Result<Self> {
        let p = crate::ble::find(adapter, crate::ble::NAME_PREFIX, scan_timeout).await?;
        let name = p.properties().await?.and_then(|x| x.local_name).unwrap_or_default();
        p.connect().await.context("connect")?;
        p.discover_services().await.context("discover services")?;
        let find = |u: Uuid| p.characteristics().into_iter().find(|c| c.uuid == u).ok_or_else(|| anyhow!("characteristic {u} missing"));
        let job_char = find(CHAR_JOB)?;
        let request_char = find(CHAR_REQUEST)?;
        let status_char = find(CHAR_STATUS)?;

        let props = Arc::new(Mutex::new(Properties::new()));
        let (tx, changed) = watch::channel(0u64);
        let mut stream = p.notifications().await?;
        p.subscribe(&status_char).await.context("subscribe indications")?;
        let props2 = props.clone();
        tokio::spawn(async move {
            let mut re = Reassembler::default();
            let mut n = 0u64;
            while let Some(v) = stream.next().await {
                if v.uuid != CHAR_STATUS {
                    continue;
                }
                for m in re.push(&v.value) {
                    match m {
                        Ok(m) => {
                            let mut p = props2.lock().unwrap();
                            for e in m.get_responses {
                                p.insert(e.id, e.value);
                            }
                            n += 1;
                            let _ = tx.send(n);
                        }
                        Err(e) => eprintln!("[proto] {e}"),
                    }
                }
            }
        });

        let mut me = Printer { peripheral: p, job_char, request_char, props, changed, name };
        me.write_request(proto::SUBSCRIBE_REQUEST).await?;
        me.wait_for_update(Duration::from_secs(10)).await.context("no property response to the subscribe request")?;
        Ok(me)
    }

    async fn write_request(&self, json: &str) -> Result<()> {
        for f in proto::request_fragments(json) {
            self.peripheral.write(&self.request_char, &f, WriteType::WithResponse).await.context("write request")?;
        }
        Ok(())
    }

    /// Waits for the next property message (any content).
    async fn wait_for_update(&mut self, timeout: Duration) -> Result<()> {
        self.changed.borrow_and_update();
        tokio::time::timeout(timeout, self.changed.changed()).await.map_err(|_| anyhow!("timeout"))??;
        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        self.peripheral.is_connected().await.unwrap_or(false)
    }

    pub fn properties(&self) -> Properties {
        self.props.lock().unwrap().clone()
    }

    pub fn status(&self) -> Status {
        Status::from_props(&self.properties())
    }

    pub async fn feed(&self) -> Result<()> {
        self.write_request(&proto::set_request(proto::PROP_FEED, "True")).await
    }

    pub async fn cut(&self) -> Result<()> {
        self.write_request(&proto::set_request(proto::PROP_CUT, "True")).await
    }

    /// Sends a job and waits for the printer's verdict on it (property 0029).
    pub async fn print(&mut self, params: &JobParams, raster: &Raster) -> Result<JobResult> {
        let bytes = job::encode(params, raster);
        let frags = job::fragments(&bytes);
        let n = frags.len();
        for f in &frags {
            self.peripheral.write(&self.job_char, f, WriteType::WithResponse).await.context("write job fragment")?;
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            let status = self.properties().get(proto::PROP_JOB_STATUS).cloned().unwrap_or_default();
            if let Some(rest) = status.strip_prefix(&format!("{}:", params.job_id)) {
                return Ok(JobResult { job_id: params.job_id.clone(), ok: rest == "Successful", status, bytes: bytes.len(), fragments: n });
            }
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                bail!("no job result within 60 s (last status: {status:?})");
            }
            if self.wait_for_update(left).await.is_err() {
                bail!("no job result within 60 s (last status: {status:?})");
            }
        }
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.peripheral.disconnect().await.context("disconnect")
    }
}

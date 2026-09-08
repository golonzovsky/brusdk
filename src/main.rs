use anyhow::Result;
use btleplug::api::{CharPropFlags, Peripheral as _};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(version, about = "Brady M511 clean-room BLE driver")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scan for the printer and print its advertisement.
    Scan {
        #[arg(long, default_value_t = 30)]
        seconds: u64,
    },
    /// Connect and dump the GATT database (services, characteristics, properties, descriptors).
    Gatt {
        #[arg(long, default_value_t = 30)]
        seconds: u64,
    },
    /// Connect, print the printer's properties and derived status, disconnect.
    Status {
        #[arg(long, default_value_t = 60)]
        seconds: u64,
    },
    /// Connect and feed.
    Feed {
        #[arg(long, default_value_t = 60)]
        seconds: u64,
    },
    /// Connect and cut.
    Cut {
        #[arg(long, default_value_t = 60)]
        seconds: u64,
    },
    /// Encode a PNG as a job; with --send, connect and print it.
    Print {
        png: PathBuf,
        #[arg(long, default_value_t = 1)]
        copies: u32,
        /// 0 = cut at end of job, 1 = after each label, 2 = never
        #[arg(long, default_value_t = 0)]
        cut: u8,
        /// Rows down from the top of the canvas (the SDK's default placement is 36)
        #[arg(long, default_value_t = 0)]
        row: i32,
        /// Columns along the tape
        #[arg(long, default_value_t = 0)]
        col: i32,
        #[arg(long)]
        send: bool,
        /// Write the encoded job bytes here
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 60)]
        seconds: u64,
    },
    /// Serve the sidecar-compatible HTTP API (GET /api/printer, POST /api/connect|disconnect|print|dryrun|feed|cut).
    Serve {
        #[arg(long, default_value_t = 5178)]
        port: u16,
    },
    /// Write the probe rasters as PNGs into a directory.
    Probes {
        #[arg(long, default_value = "probes")]
        out: PathBuf,
        /// Base width in pixels (label length at 300 dpi)
        #[arg(long, default_value_t = 64)]
        width: u32,
        /// Image height in rows
        #[arg(long, default_value_t = 432)]
        rows: u32,
    },
}

async fn connect(seconds: u64) -> Result<brusdk::printer::Printer> {
    let a = brusdk::ble::adapter().await?;
    let p = brusdk::printer::Printer::connect(&a, Duration::from_secs(seconds)).await?;
    eprintln!("connected to {}", p.name);
    Ok(p)
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Scan { seconds } => {
            let a = brusdk::ble::adapter().await?;
            let p = brusdk::ble::find(&a, brusdk::ble::NAME_PREFIX, Duration::from_secs(seconds)).await?;
            println!("{:#?}", p.properties().await?);
        }
        Cmd::Gatt { seconds } => {
            let a = brusdk::ble::adapter().await?;
            let p = brusdk::ble::find(&a, brusdk::ble::NAME_PREFIX, Duration::from_secs(seconds)).await?;
            println!("{:#?}", p.properties().await?);
            p.connect().await?;
            p.discover_services().await?;
            for s in p.services() {
                println!("service {} primary={}", s.uuid, s.primary);
                for c in &s.characteristics {
                    println!("  char {} props={:?}", c.uuid, c.properties);
                    for d in &c.descriptors {
                        println!("    desc {}", d.uuid);
                    }
                    if c.properties.contains(CharPropFlags::READ) {
                        match p.read(c).await {
                            Ok(v) => println!("    value {} {:?}", hex::encode(&v), String::from_utf8_lossy(&v)),
                            Err(e) => println!("    read error {e}"),
                        }
                    }
                }
            }
            p.disconnect().await?;
        }
        Cmd::Status { seconds } => {
            let p = connect(seconds).await?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "name": p.name, "properties": p.properties(), "status": p.status() }))?);
            p.disconnect().await?;
        }
        Cmd::Feed { seconds } => {
            let p = connect(seconds).await?;
            p.feed().await?;
            tokio::time::sleep(Duration::from_secs(3)).await;
            println!("{}", serde_json::to_string_pretty(&p.status())?);
            p.disconnect().await?;
        }
        Cmd::Cut { seconds } => {
            let p = connect(seconds).await?;
            p.cut().await?;
            tokio::time::sleep(Duration::from_secs(3)).await;
            println!("{}", serde_json::to_string_pretty(&p.status())?);
            p.disconnect().await?;
        }
        Cmd::Print { png, copies, cut, row, col, send, out, seconds } => {
            let img = image::open(&png)?.to_luma8();
            let mut params = brusdk::job::JobParams::new(brusdk::job::new_job_id(), "");
            params.copies = copies;
            params.cut = cut;
            if !send {
                let raster = brusdk::job::Raster::place(&img, 450, col, row);
                params.supply_prefix = brusdk::supply::job_prefix(brusdk::supply::SUPPLIES[0].name);
                let bytes = brusdk::job::encode(&params, &raster);
                let frags = brusdk::job::fragments(&bytes);
                println!("job {} bytes, {} fragments, id {}", bytes.len(), frags.len(), params.job_id);
                if let Some(out) = out {
                    std::fs::write(out, &bytes)?;
                }
                return Ok(());
            }
            let mut p = connect(seconds).await?;
            let st = p.status();
            let part = st.supply_part_number.clone().unwrap_or_default();
            let supply = brusdk::supply::lookup(&part).ok_or_else(|| anyhow::anyhow!("unknown supply part number {part:?}; add it to src/supply.rs"))?;
            params.supply_prefix = brusdk::supply::job_prefix(supply.name);
            let raster = brusdk::job::Raster::place(&img, brusdk::supply::canvas_rows(supply.width_in), col, row);
            let res = p.print(&params, &raster).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
            tokio::time::sleep(Duration::from_secs(1)).await;
            p.disconnect().await?;
        }
        Cmd::Serve { port } => brusdk::server::serve(port).await?,
        Cmd::Probes { out, width, rows } => {
            std::fs::create_dir_all(&out)?;
            for pr in brusdk::probes::probes(width, rows) {
                let img = brusdk::probes::render(&pr);
                let path = out.join(format!("{}.png", pr.name));
                img.save(&path)?;
                println!("{} {}x{}", path.display(), img.width(), img.height());
            }
        }
    }
    Ok(())
}

//! cargo run --release --example print -- label.png
use anyhow::Result;
use brusdk::job::{JobParams, Raster, new_job_id};
use brusdk::printer::Printer;
use brusdk::supply;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let png = std::env::args().nth(1).expect("usage: print <label.png>");
    let img = image::open(png)?.to_luma8();

    let adapter = brusdk::ble::adapter().await?;
    let mut printer = Printer::connect(&adapter, Duration::from_secs(60)).await?;
    let status = printer.status();
    println!("{}: {:?}", printer.name, status);

    let part = status.supply_part_number.unwrap_or_default();
    let cartridge = supply::lookup(&part).ok_or_else(|| anyhow::anyhow!("unknown cartridge {part}"))?;
    let raster = Raster::place(&img, supply::canvas_rows(cartridge.width_in), 0, 0);
    let mut job = JobParams::new(new_job_id(), supply::job_prefix(cartridge.name));
    job.copies = 1;
    job.cut = 0;
    let result = printer.print(&job, &raster).await?;
    println!("{result:?}");

    printer.disconnect().await
}

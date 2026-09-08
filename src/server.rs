//! HTTP surface compatible with the label studio's Node sidecar (server/printer.mjs + index.mjs).

use crate::job::{JobParams, Raster, new_job_id};
use crate::printer::Printer;
use crate::supply;
use anyhow::{Result, anyhow};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::{get, post}};
use base64::Engine as _;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// The SDK's head window (1.44 in) as the studio knows it; the canvas itself is the full supply width.
const PRINTABLE_HEIGHT_IN: f64 = 1.44;
/// The SDK placed images 36 rows down; the studio passes -0.12 in to cancel that. Keep that contract.
const SDK_DEFAULT_ROW: i32 = 36;
const DEFAULT_X_OFFSET_IN: f64 = -0.12;

/// Property 0001 is a class, not a number. "High" is the only value observed; the SDK showed 100 % for it.
/// Other classes have not been seen, so they stay unmapped rather than guessed.
fn battery_percent(class: Option<&str>) -> Option<u32> {
    match class {
        Some("High") => Some(100),
        _ => None,
    }
}

pub struct App {
    printer: Mutex<Option<Printer>>,
    busy: std::sync::Mutex<Option<&'static str>>,
    last_error: std::sync::Mutex<Option<String>>,
}

type S = State<Arc<App>>;

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}"))
    }
}

struct Busy<'a>(&'a App);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        *self.0.busy.lock().unwrap() = None;
    }
}

fn claim<'a>(app: &'a App, what: &'static str) -> Result<Busy<'a>, ApiError> {
    let mut b = app.busy.lock().unwrap();
    if let Some(cur) = *b {
        return Err(ApiError(StatusCode::CONFLICT, format!("printer is busy ({cur})")));
    }
    *b = Some(what);
    Ok(Busy(app))
}

async fn snapshot(app: &App) -> Value {
    let guard = app.printer.lock().await;
    let mut printer = None;
    if let Some(p) = guard.as_ref() {
        if p.is_connected().await {
            let st = p.status();
            let sup = st.supply_part_number.as_deref().and_then(supply::lookup);
            let width = sup.map(|s| s.width_in).or(st.supply_width_in);
            printer = Some(json!({
                "printerName": p.name, "printerModel": "M511", "status": "Connected", "firmwareVersion": st.firmware_version,
                "supplyName": sup.map(|s| s.name), "supplyYNumber": st.supply_part_number, "supplyWidth": width, "supplyHeight": Value::Null,
                "mediaIsDieCut": sup.map(|s| s.die_cut), "dotsPerInch": supply::DPI, "zoneDimensions": [],
                "supplyRemainingPercentage": st.supply_remaining_pct, "batteryLevel": st.battery, "batteryLevelPercentage": battery_percent(st.battery.as_deref()),
                "isAcConnected": Value::Null, "message": st.last_job_status, "messageTitle": Value::Null,
                "errorSeverity": if st.last_job_failed { "Error" } else { "" },
                "printableHeightIn": PRINTABLE_HEIGHT_IN, "xOffsetIn": DEFAULT_X_OFFSET_IN, "properties": p.properties(),
            }));
        } else {
            *app.last_error.lock().unwrap() = Some("Printer dropped the Bluetooth link (it sleeps when idle). Wake it and connect again.".into());
        }
    }
    json!({ "connected": printer.is_some(), "busy": *app.busy.lock().unwrap(), "lastError": *app.last_error.lock().unwrap(), "printer": printer })
}

async fn get_printer(State(app): S) -> Json<Value> {
    Json(snapshot(&app).await)
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ConnectReq {
    scan_seconds: Option<f64>,
}

async fn connect(State(app): S, body: Option<Json<ConnectReq>>) -> Result<Json<Value>, ApiError> {
    let _busy = claim(&app, "connecting")?;
    let secs = body.and_then(|b| b.scan_seconds).unwrap_or(30.0).clamp(1.0, 300.0);
    let mut guard = app.printer.lock().await;
    let live = match guard.as_ref() {
        Some(p) => p.is_connected().await,
        None => false,
    };
    if !live {
        *guard = None;
        *app.last_error.lock().unwrap() = None;
        let adapter = crate::ble::adapter().await?;
        match Printer::connect(&adapter, Duration::from_secs(secs as u64)).await {
            Ok(p) => *guard = Some(p),
            Err(e) => *app.last_error.lock().unwrap() = Some(format!("{e:#}")),
        }
    }
    drop(guard);
    Ok(Json(snapshot(&app).await))
}

async fn disconnect(State(app): S) -> Result<Json<Value>, ApiError> {
    let _busy = claim(&app, "disconnecting")?;
    if let Some(p) = app.printer.lock().await.take() {
        let _ = p.disconnect().await;
    }
    Ok(Json(snapshot(&app).await))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrintReq {
    png_base64: String,
    #[serde(default = "one")]
    copies: u32,
    #[serde(default)]
    cut_option: u8,
    x_offset_in: Option<f64>,
    y_offset_in: Option<f64>,
}

fn one() -> u32 {
    1
}

async fn run_job(app: &App, req: PrintReq, dry: bool) -> Result<Value, ApiError> {
    let png = base64::engine::general_purpose::STANDARD.decode(req.png_base64.trim()).map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("pngBase64: {e}")))?;
    let img = image::load_from_memory(&png).map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("png: {e}")))?.to_luma8();
    let x = req.x_offset_in.unwrap_or(DEFAULT_X_OFFSET_IN);
    let y = req.y_offset_in.unwrap_or(0.0);
    let dpi = supply::DPI as f64;
    let mut guard = app.printer.lock().await;
    let (supply, connected) = match guard.as_ref() {
        Some(p) if p.is_connected().await => {
            let part = p.status().supply_part_number.unwrap_or_default();
            (supply::lookup(&part).ok_or_else(|| anyhow!("unknown supply part number {part:?}; add it to src/supply.rs"))?, true)
        }
        _ => (&supply::SUPPLIES[0], false),
    };
    if !dry && !connected {
        return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "printer not connected: wake it (power button) and press Connect".into()));
    }
    let rows = supply::canvas_rows(supply.width_in);
    let raster = Raster::place(&img, rows, (y * dpi).round() as i32, SDK_DEFAULT_ROW + (x * dpi).round() as i32);
    let mut params = JobParams::new(new_job_id(), supply::job_prefix(supply.name));
    params.copies = req.copies.max(1);
    params.cut = req.cut_option;
    let ink = raster.ink_rows();
    let head_rows = (PRINTABLE_HEIGHT_IN * dpi).round() as u32;
    let mut png_out = Vec::new();
    raster.to_image().write_to(&mut std::io::Cursor::new(&mut png_out), image::ImageFormat::Png).map_err(|e| anyhow!(e))?;
    let (ok, error, message) = if dry {
        (true, None, None)
    } else if ink.is_none() {
        (false, Some("nothing to print: the raster is blank (the printer rejects blank jobs)".to_string()), None)
    } else {
        let p = guard.as_mut().unwrap();
        match p.print(&params, &raster).await {
            Ok(r) => (r.ok, if r.ok { None } else { Some(r.status.clone()) }, Some(r.status)),
            Err(e) => (false, Some(format!("{e:#}")), None),
        }
    };
    Ok(json!({
        "ok": ok, "error": error, "dry": dry,
        "image": { "width": img.width(), "height": img.height() },
        "raster": { "width": raster.cols, "height": raster.rows, "png": base64::engine::general_purpose::STANDARD.encode(&png_out) },
        "inkRows": ink.map(|(a, b)| [a, b]), "clipped": ink.is_some_and(|(_, b)| b >= head_rows),
        "xOffsetIn": x, "yOffsetIn": y, "printerMessage": message.unwrap_or_default(),
        "jobId": params.job_id, "jobBytes": crate::job::encode(&params, &raster).len(),
    }))
}

async fn print(State(app): S, Json(req): Json<PrintReq>) -> Result<Json<Value>, ApiError> {
    let _busy = claim(&app, "printing")?;
    Ok(Json(run_job(&app, req, false).await?))
}

async fn dryrun(State(app): S, Json(req): Json<PrintReq>) -> Result<Json<Value>, ApiError> {
    let _busy = claim(&app, "rendering")?;
    Ok(Json(run_job(&app, req, true).await?))
}

async fn with_printer(app: &App, what: &'static str, f: impl AsyncFnOnce(&Printer) -> Result<()>) -> Result<Json<Value>, ApiError> {
    let _busy = claim(app, what)?;
    let guard = app.printer.lock().await;
    match guard.as_ref() {
        Some(p) if p.is_connected().await => {
            f(p).await?;
            Ok(Json(json!({ "ok": true })))
        }
        _ => Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "printer not connected: wake it (power button) and press Connect".into())),
    }
}

async fn feed(State(app): S) -> Result<Json<Value>, ApiError> {
    with_printer(&app, "feeding", async |p| p.feed().await).await
}

async fn cut(State(app): S) -> Result<Json<Value>, ApiError> {
    with_printer(&app, "cutting", async |p| p.cut().await).await
}

pub async fn serve(port: u16) -> Result<()> {
    let app = Arc::new(App { printer: Mutex::new(None), busy: std::sync::Mutex::new(None), last_error: std::sync::Mutex::new(None) });
    let router = Router::new()
        .route("/api/printer", get(get_printer))
        .route("/api/connect", post(connect))
        .route("/api/disconnect", post(disconnect))
        .route("/api/print", post(print))
        .route("/api/dryrun", post(dryrun))
        .route("/api/feed", post(feed))
        .route("/api/cut", post(cut))
        .with_state(app);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!("brusdk printer server on http://127.0.0.1:{port}");
    axum::serve(listener, router).await?;
    Ok(())
}

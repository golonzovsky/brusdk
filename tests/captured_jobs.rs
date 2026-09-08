//! Byte-exact check of the encoder against every job captured from the SDK in session 1.

use brusdk::job::{JobParams, Raster, encode, fragments};
use std::path::Path;

struct Captured {
    fragments: Vec<Vec<u8>>,
}

fn load(session: &str, name: &str) -> Captured {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("captures/{session}"));
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(&format!("-{name}.jsonl")))
        .collect();
    files.sort();
    let text = std::fs::read_to_string(files.last().expect("capture file")).unwrap();
    let mut fragments = Vec::new();
    for line in text.lines() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        if v["ev"] == "tx" && v["char"] == "7d9d9a4d" {
            fragments.push(hex::decode(v["hex"].as_str().unwrap()).unwrap());
        }
    }
    Captured { fragments }
}

fn check(name: &str) {
    check_with("2026-09-08-s1", name, name, |p| p, 0, 0);
}

fn check_with(session: &str, name: &str, probe: &str, tweak: fn(JobParams) -> JobParams, col: i32, row: i32) {
    let cap = load(session, name);
    let job: Vec<u8> = cap.fragments.iter().flat_map(|f| f[3..].to_vec()).collect();
    let job_id = std::str::from_utf8(&job[4..36]).unwrap().to_string();
    let img = image::open(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("captures/probes/{probe}.png")))
        .unwrap()
        .to_luma8();
    let raster = Raster::place(&img, 450, col, row);
    let ours = encode(&tweak(JobParams::new(job_id, "M5C-1500-5")), &raster);
    assert_eq!(hex::encode(&ours), hex::encode(&job), "{name}: job bytes differ");
    assert_eq!(fragments(&ours), cap.fragments, "{name}: fragmentation differs");
}

#[test]
fn white_150() { check("white-150"); }
#[test]
fn dot_r0c0_150() { check("dot-r0c0-150"); }
#[test]
fn dot_r1c0_150() { check("dot-r1c0-150"); }
#[test]
fn dot_r0c1_150() { check("dot-r0c1-150"); }
#[test]
fn dot_r8c0_150() { check("dot-r8c0-150"); }
#[test]
fn dot_r431clast_150() { check("dot-r431cLast-150"); }
#[test]
fn col5_150() { check("col5-150"); }
#[test]
fn row100_150() { check("row100-150"); }
#[test]
fn altrows_150() { check("altrows-150"); }
#[test]
fn band_r100_200_150() { check("band-r100-200-150"); }
#[test]
fn checker_150() { check("checker-150"); }

#[test]
fn s2_copies2() { check_with("2026-09-08-s2", "copies2", "dot-r0c0-150", |mut p| { p.copies = 2; p }, 0, 0); }
#[test]
fn s2_cut1() { check_with("2026-09-08-s2", "cut1", "dot-r0c0-150", |mut p| { p.cut = 1; p }, 0, 0); }
#[test]
fn s2_cut2() { check_with("2026-09-08-s2", "cut2", "dot-r0c0-150", |mut p| { p.cut = 2; p }, 0, 0); }
/// SDK x offset 0 (instead of -0.12 in) puts the image 36 rows down the canvas.
#[test]
fn s2_x0() { check_with("2026-09-08-s2", "x0", "dot-r0c0-150", |p| p, 0, 36); }
/// SDK y offset 0.1 in moves the image 30 columns along the tape.
#[test]
fn s2_y0_1() { check_with("2026-09-08-s2", "y0.1", "dot-r0c0-150", |p| p, 30, 0); }
#[test]
fn s2_corners_p1_150() { check_with("2026-09-08-s2", "corners-p1-150", "corners-p1-150", |p| p, 0, 0); }

/// Narrow supply (M4C-250-7641-YL, 0.355 in): the SDK's canvas is 149 x 106 (its own raster PNG is the input here),
/// height field 0106, dot at row 0 with a zero x offset, supply prefix "M4C-250-76".
#[test]
fn s4_m4c_dot() {
    let cap = load("2026-09-08-s4", "m4c-dot-r0c0-150x107");
    let job: Vec<u8> = cap.fragments.iter().flat_map(|f| f[3..].to_vec()).collect();
    let job_id = std::str::from_utf8(&job[4..36]).unwrap().to_string();
    let img = image::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("captures/2026-09-08-s4/m4c-dot-r0c0-150x107-raster.png")).unwrap().to_luma8();
    let width_in = 0.355;
    assert_eq!(brusdk::supply::canvas_rows(width_in), 106);
    assert_eq!(brusdk::supply::default_row(width_in), 0);
    assert_eq!(brusdk::supply::default_row(1.5), 36);
    let raster = Raster::place(&img, brusdk::supply::canvas_rows(width_in), 0, brusdk::supply::default_row(width_in));
    let ours = encode(&JobParams::new(job_id, brusdk::supply::job_prefix("M4C-250-7641-YL")), &raster);
    assert_eq!(hex::encode(&ours), hex::encode(&job));
}

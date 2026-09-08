//! Print job encoding for the `7d9d9a4d` characteristic. See PROTOCOL.md section 4.

use std::io::Write as _;

/// 1-bit raster: `rows` across the tape, `cols` along it; `black(x, y)` with y = row.
pub struct Raster {
    pub cols: u32,
    pub rows: u32,
    bits: Vec<bool>,
}

impl Raster {
    pub fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows, bits: vec![false; (cols * rows) as usize] }
    }

    pub fn set(&mut self, x: u32, y: u32, black: bool) {
        self.bits[(y * self.cols + x) as usize] = black;
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        self.bits[(y * self.cols + x) as usize]
    }

    /// Place a grayscale image (0 = black) at the top-left of a `rows`-tall raster; pixels < 128 are black.
    pub fn from_gray(img: &image::GrayImage, rows: u32) -> Self {
        let mut r = Self::new(img.width(), rows);
        for (x, y, p) in img.enumerate_pixels() {
            if y < rows {
                r.set(x, y, p.0[0] < 128);
            }
        }
        r
    }

    /// Place a 1-bit image on a `rows`-tall canvas with its top-left at (`col`, `row`); pixels outside are dropped.
    /// Canvas width = image width (columns shifted off the right edge are lost, as in the captured jobs).
    pub fn place(img: &image::GrayImage, rows: u32, col: i32, row: i32) -> Self {
        let mut r = Self::new(img.width(), rows);
        for (x, y, p) in img.enumerate_pixels() {
            let (cx, cy) = (x as i32 + col, y as i32 + row);
            if p.0[0] < 128 && cx >= 0 && cy >= 0 && (cx as u32) < r.cols && (cy as u32) < rows {
                r.set(cx as u32, cy as u32, true);
            }
        }
        r
    }

    pub fn to_image(&self) -> image::GrayImage {
        image::GrayImage::from_fn(self.cols, self.rows, |x, y| image::Luma([if self.get(x, y) { 0 } else { 255 }]))
    }

    /// First and last row containing ink, if any.
    pub fn ink_rows(&self) -> Option<(u32, u32)> {
        let rows: Vec<u32> = (0..self.rows).filter(|&y| (0..self.cols).any(|x| self.get(x, y))).collect();
        Some((*rows.first()?, *rows.last()?))
    }

    fn column(&self, x: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; self.rows.div_ceil(8) as usize];
        for y in 0..self.rows {
            if self.get(x, y) {
                bytes[(y / 8) as usize] |= 0x80 >> (y % 8);
            }
        }
        bytes
    }
}

pub const MAX_FRAGMENT_PAYLOAD: usize = 148;

#[derive(Clone, Debug)]
pub struct JobParams {
    /// 32 lowercase hex digits (a UUIDv4 without dashes).
    pub job_id: String,
    /// First 10 characters of the supply name, e.g. "M5C-1500-5".
    pub supply_prefix: String,
    pub d: i32,
    /// Number of copies: the `C` field, and the label block is repeated that many times.
    pub copies: u32,
    pub c_lower: u8,
    /// `M` byte: 0 = cut at end of job, 1 = cut after each label, 2 = never cut.
    pub cut: u8,
    pub p: i32,
    pub o: i32,
    pub o_upper: i32,
    pub b: i32,
    pub m: u8,
    pub label_name: String,
}

impl JobParams {
    pub fn new(job_id: impl Into<String>, supply_prefix: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
            supply_prefix: supply_prefix.into(),
            d: 1,
            copies: 1,
            c_lower: 0,
            cut: 0,
            p: 0,
            o: 0,
            o_upper: 0,
            b: 0,
            m: 0,
            label_name: "BUlbl0".into(),
        }
    }
}

fn cmd(out: &mut Vec<u8>, letter: u8) {
    out.extend_from_slice(&[0x02, letter]);
}

fn cmd_k(out: &mut Vec<u8>, sub: u16) {
    cmd(out, b'K');
    out.extend_from_slice(&sub.to_be_bytes());
}

fn signed(out: &mut Vec<u8>, v: i32, digits: usize) {
    let _ = write!(out, "{}{:0width$}", if v < 0 { '-' } else { '+' }, v.abs(), width = digits);
}

/// Encode one column: literal bytes with trailing zeros trimmed, or run lengths when strictly shorter.
fn encode_column(bytes: &[u8], rows: u32) -> Vec<u8> {
    let lit_len = bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let mut runs = Vec::new();
    let mut y = 0;
    while y < rows {
        let black = bytes[(y / 8) as usize] & (0x80 >> (y % 8)) != 0;
        let start = y;
        while y < rows && (bytes[(y / 8) as usize] & (0x80 >> (y % 8)) != 0) == black {
            y += 1;
        }
        if !black && y == rows {
            break;
        }
        let mut len = y - start;
        while len > 0 {
            let n = len.min(128);
            runs.push(if black { 0x80 } else { 0 } | (n - 1) as u8);
            len -= n;
        }
    }
    let mut out = Vec::new();
    if runs.len() < lit_len {
        out.push(0x81);
        out.push(runs.len() as u8);
        out.extend_from_slice(&runs);
    } else {
        out.push(0x80);
        out.push(lit_len as u8);
        out.extend_from_slice(&bytes[..lit_len]);
    }
    out
}

pub fn encode(p: &JobParams, r: &Raster) -> Vec<u8> {
    let mut out = Vec::new();
    for copy in 0..p.copies.max(1) {
        cmd_k(&mut out, 0x000a);
        out.extend_from_slice(p.job_id.as_bytes());
        out.push(0x0d);
        if copy == 0 {
            cmd_k(&mut out, 0x0009);
            out.extend_from_slice(p.supply_prefix.as_bytes());
            out.push(0x0d);
            cmd(&mut out, b'D');
            signed(&mut out, p.d, 4);
            cmd(&mut out, b'C');
            signed(&mut out, p.copies as i32, 4);
            cmd(&mut out, b'c');
            out.push(p.c_lower);
            for (letter, v) in [(b'p', p.p), (b'o', p.o), (b'O', p.o_upper), (b'b', p.b)] {
                cmd(&mut out, letter);
                signed(&mut out, v, 2);
            }
            cmd(&mut out, b'M');
            out.push(p.cut);
        }
        cmd_k(&mut out, 0x000c);
        let _ = write!(out, "{:04}{:04}", r.rows, r.cols);
        cmd(&mut out, b'A');
        cmd(&mut out, b'Q');
        cmd(&mut out, b'a');
        cmd(&mut out, b'I');
        out.extend_from_slice(p.label_name.as_bytes());
        out.push(0x0d);
        encode_raster(&mut out, r);
        cmd(&mut out, b'a');
        cmd(&mut out, b'G');
        cmd(&mut out, b'A');
        cmd_k(&mut out, 0x000b);
    }
    out
}

fn encode_raster(out: &mut Vec<u8>, r: &Raster) {
    out.extend_from_slice(&[b'X', 0, 0, b'Y', 0, 0]);
    let mut cursor: u32 = 0;
    for yidx in 0..r.cols {
        let x = r.cols - 1 - yidx;
        let bytes = r.column(x);
        if bytes.iter().all(|&b| b == 0) {
            continue;
        }
        if cursor != yidx {
            out.push(b'Y');
            out.extend_from_slice(&(yidx as u16).to_le_bytes());
        }
        out.extend_from_slice(&encode_column(&bytes, r.rows));
        cursor = yidx + 1;
    }
    if cursor != 0 && cursor != r.cols {
        out.push(b'Y');
        out.extend_from_slice(&(r.cols as u16).to_le_bytes());
    }
    out.extend_from_slice(&[0xff, 0xff, 0x0d]);
}

/// Split a job into write fragments: `01 idx 00` + up to 148 bytes; `02` on every 16th fragment (idx % 16 == 15), `03` on the last.
pub fn fragments(job: &[u8]) -> Vec<Vec<u8>> {
    let chunks: Vec<&[u8]> = job.chunks(MAX_FRAGMENT_PAYLOAD).collect();
    chunks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let kind = if i + 1 == chunks.len() { 0x03 } else if i % 16 == 15 { 0x02 } else { 0x01 };
            let mut f = vec![kind, i as u8, 0x00];
            f.extend_from_slice(c);
            f
        })
        .collect()
}

pub fn new_job_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

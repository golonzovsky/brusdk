//! Probe rasters for protocol capture: tiny 1-bit images whose content makes the wire encoding obvious.

use image::{GrayImage, Luma};

pub const ROWS: u32 = 432;

pub struct Probe {
    pub name: &'static str,
    pub width: u32,
    /// (x, y, width) -> black
    pub paint: fn(u32, u32, u32) -> bool,
}


pub fn probes(w: u32) -> Vec<Probe> {
    let n = |base: &str| -> &'static str { Box::leak(format!("{base}-{w}").into_boxed_str()) };
    vec![
        Probe { name: n("white"), width: w, paint: |_, _, _| false },
        Probe { name: n("dot-r0c0"), width: w, paint: |x, y, _| x == 0 && y == 0 },
        Probe { name: n("dot-r1c0"), width: w, paint: |x, y, _| x == 0 && y == 1 },
        Probe { name: n("dot-r0c1"), width: w, paint: |x, y, _| x == 1 && y == 0 },
        Probe { name: n("dot-r8c0"), width: w, paint: |x, y, _| x == 0 && y == 8 },
        Probe { name: n("dot-r431cLast"), width: w, paint: |x, y, w| x == w - 1 && y == 431 },
        Probe { name: n("col5"), width: w, paint: |x, _, _| x == 5 },
        Probe { name: n("row100"), width: w, paint: |_, y, _| y == 100 },
        Probe { name: n("altrows"), width: w, paint: |_, y, _| y % 2 == 0 },
        Probe { name: n("band-r100-200"), width: w, paint: |_, y, _| (100..200).contains(&y) },
        Probe { name: n("checker"), width: w, paint: |x, y, _| (x + y) % 2 == 0 },
        Probe { name: n("corners-m2"), width: w - 2, paint: |x, y, w| (x == 0 && y == 0) || (x == w - 1 && y == 431) },
        Probe { name: n("corners-m1"), width: w - 1, paint: |x, y, w| (x == 0 && y == 0) || (x == w - 1 && y == 431) },
        Probe { name: n("corners-p1"), width: w + 1, paint: |x, y, w| (x == 0 && y == 0) || (x == w - 1 && y == 431) },
    ]
}

pub fn render(p: &Probe) -> GrayImage {
    let mut img = GrayImage::from_pixel(p.width, ROWS, Luma([255]));
    for y in 0..ROWS {
        for x in 0..p.width {
            if (p.paint)(x, y, p.width) {
                img.put_pixel(x, y, Luma([0]));
            }
        }
    }
    img
}

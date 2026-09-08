//! Supply part numbers seen on the wire (property 004D) mapped to what the SDK reported for them.
//! The printer never sends the name, dpi or die-cut flag; these come from Alex's cartridges as observed.

pub struct Supply {
    pub part_number: &'static str,
    pub name: &'static str,
    pub width_in: f64,
    pub die_cut: bool,
}

pub const SUPPLIES: &[Supply] = &[
    Supply { part_number: "5153508", name: "M5C-1500-595-WT-BK", width_in: 1.5, die_cut: false },
    Supply { part_number: "5072987", name: "M5C-1500-595-OR-BK", width_in: 1.5, die_cut: false },
    Supply { part_number: "5072986", name: "M5C-1500-595-CL-BK", width_in: 1.5, die_cut: false },
    Supply { part_number: "5072905", name: "M5C-1500-595-CL-WT", width_in: 1.5, die_cut: false },
    Supply { part_number: "5072903", name: "M5C-1500-595-BK-WT", width_in: 1.5, die_cut: false },
    Supply { part_number: "5073028", name: "M4C-250-7641-YL", width_in: 0.355, die_cut: false },
];

pub const DPI: u32 = 300;

/// Head window the SDK works with (1.44 in).
pub const HEAD_IN: f64 = 1.44;

/// Canvas height = supply width at 300 dpi, truncated: 1.5 in -> 450, 0.355 in -> 106 (captures s1, s4).
pub fn canvas_rows(width_in: f64) -> u32 {
    (width_in * DPI as f64) as u32
}

/// Row the SDK puts the image's top on with a zero x offset: 36 on 1.5 in, 0 on 0.355 in (s2/x0, s4).
/// Fits 2 * (width - 1.44 in), the rule the studio's default x offset cancels.
pub fn default_row(width_in: f64) -> i32 {
    if width_in > HEAD_IN { (2.0 * (width_in - HEAD_IN) * DPI as f64).round() as i32 } else { 0 }
}

pub fn lookup(part_number: &str) -> Option<&'static Supply> {
    SUPPLIES.iter().find(|s| s.part_number == part_number)
}

/// First 10 characters of the supply name, as the job header wants it.
pub fn job_prefix(name: &str) -> String {
    name.chars().take(10).collect()
}

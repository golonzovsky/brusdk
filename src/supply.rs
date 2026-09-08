//! Supply part numbers seen on the wire (property 004D) mapped to what the SDK reported for them.
//! The printer never sends the name, dpi or die-cut flag; these come from Alex's cartridges as observed.

pub struct Supply {
    pub part_number: &'static str,
    pub name: &'static str,
    pub width_in: f64,
    pub die_cut: bool,
}

pub const SUPPLIES: &[Supply] = &[Supply { part_number: "5153508", name: "M5C-1500-595-WT-BK", width_in: 1.5, die_cut: false }];

pub const DPI: u32 = 300;

/// Rows the print head covers on the 1.5 in supply: the SDK's canvas height.
pub fn canvas_rows(width_in: f64) -> u32 {
    (width_in * DPI as f64).round() as u32
}

pub fn lookup(part_number: &str) -> Option<&'static Supply> {
    SUPPLIES.iter().find(|s| s.part_number == part_number)
}

/// First 10 characters of the supply name, as the job header wants it.
pub fn job_prefix(name: &str) -> String {
    name.chars().take(10).collect()
}

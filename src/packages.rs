use std::io::{self, BufRead};

use anyhow::{Context, Result};

const DEFAULT_PACKAGES: &str = include_str!("../packages/base.txt");
const HYPRLAND_PACKAGES: &str = include_str!("../packages/hyprland.txt");

pub fn load_base_packages(path: Option<&str>) -> Result<Vec<String>> {
    match path {
        Some(path) => {
            let file = std::fs::File::open(path).with_context(|| format!("open {}", path))?;
            let reader = io::BufReader::new(file);
            parse_packages(reader, Some(path))
        }
        None => parse_packages(DEFAULT_PACKAGES.as_bytes(), None),
    }
}

pub fn load_hyprland_packages() -> Result<Vec<String>> {
    parse_packages(HYPRLAND_PACKAGES.as_bytes(), Some("hyprland.txt"))
}

pub fn parse_packages_arg() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--packages-file" {
            return args.next();
        }
        if let Some(value) = arg.strip_prefix("--packages-file=") {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_packages<R: io::Read>(reader: R, source: Option<&str>) -> Result<Vec<String>> {
    let buf = io::BufReader::new(reader);
    let mut packages = Vec::new();
    for line in buf.lines().flatten() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        packages.push(trimmed.to_string());
    }
    if packages.is_empty() {
        let source = source.unwrap_or("embedded package list");
        anyhow::bail!("no packages found in {}", source);
    }
    Ok(packages)
}

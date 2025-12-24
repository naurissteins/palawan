use std::collections::HashSet;
use std::fs;
use std::process::{Command, Stdio};

use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuVendor {
    Amd,
    Intel,
    Nvidia,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvidiaVariant {
    Open,
    Proprietary,
    Nouveau,
}

pub fn detect_gpu_vendors() -> Result<HashSet<GpuVendor>> {
    let mut vendors = HashSet::new();
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("card") {
                continue;
            }
            let vendor_path = entry.path().join("device/vendor");
            if let Ok(contents) = fs::read_to_string(vendor_path) {
                if let Some(vendor) = parse_vendor_id(contents.trim()) {
                    vendors.insert(vendor);
                }
            }
        }
    }

    if vendors.is_empty() {
        if let Ok(output) = Command::new("lspci").arg("-nn").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if !is_gpu_line(line) {
                    continue;
                }
                if let Some(vendor_id) = parse_vendor_from_lspci(line) {
                    if let Some(vendor) = parse_vendor_id(&vendor_id) {
                        vendors.insert(vendor);
                    }
                }
            }
        }
    }

    Ok(vendors)
}

pub fn detect_installed_nvidia_variant() -> Option<NvidiaVariant> {
    if is_pkg_installed("nvidia-open-dkms") {
        return Some(NvidiaVariant::Open);
    }
    if is_pkg_installed("nvidia-dkms") {
        return Some(NvidiaVariant::Proprietary);
    }
    if is_pkg_installed("xf86-video-nouveau") || is_pkg_installed("vulkan-nouveau") {
        return Some(NvidiaVariant::Nouveau);
    }
    None
}

pub fn nvidia_driver_installed() -> bool {
    detect_installed_nvidia_variant().is_some()
}

pub fn driver_packages(
    vendors: &HashSet<GpuVendor>,
    nvidia_variant: Option<NvidiaVariant>,
) -> Vec<String> {
    let mut packages = Vec::new();
    if vendors.contains(&GpuVendor::Amd) {
        extend_unique(&mut packages, &[
            "libva-mesa-driver",
            "mesa",
            "vulkan-radeon",
            "xf86-video-amdgpu",
            "xf86-video-ati",
        ]);
    }
    if vendors.contains(&GpuVendor::Intel) {
        extend_unique(&mut packages, &[
            "intel-media-driver",
            "libva-intel-driver",
            "mesa",
            "vulkan-intel",
        ]);
    }
    if vendors.contains(&GpuVendor::Nvidia) {
        if let Some(variant) = nvidia_variant {
            match variant {
                NvidiaVariant::Open => extend_unique(&mut packages, &[
                    "dkms",
                    "libva-nvidia-driver",
                    "nvidia-open-dkms",
                ]),
                NvidiaVariant::Proprietary => extend_unique(&mut packages, &[
                    "dkms",
                    "libva-nvidia-driver",
                    "nvidia-dkms",
                ]),
                NvidiaVariant::Nouveau => extend_unique(&mut packages, &[
                    "libva-mesa-driver",
                    "mesa",
                    "vulkan-nouveau",
                    "xf86-video-nouveau",
                ]),
            }
        }
    }
    packages
}

pub fn format_gpu_summary(
    vendors: &HashSet<GpuVendor>,
    nvidia_variant: Option<NvidiaVariant>,
    installed_variant: Option<NvidiaVariant>,
) -> Option<String> {
    if vendors.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if vendors.contains(&GpuVendor::Amd) {
        parts.push("AMD");
    }
    if vendors.contains(&GpuVendor::Intel) {
        parts.push("Intel");
    }
    if vendors.contains(&GpuVendor::Nvidia) {
        parts.push("NVIDIA");
    }
    let mut line = format!("Detected GPU: {}", parts.join(", "));
    if let Some(variant) = installed_variant {
        line.push_str(&format!(" (NVIDIA driver: {})", nvidia_variant_label(variant)));
    } else if let Some(variant) = nvidia_variant {
        line.push_str(&format!(" (NVIDIA driver: {})", nvidia_variant_label(variant)));
    }
    Some(line)
}

pub fn nvidia_variant_label(variant: NvidiaVariant) -> &'static str {
    match variant {
        NvidiaVariant::Open => "open",
        NvidiaVariant::Proprietary => "proprietary",
        NvidiaVariant::Nouveau => "nouveau",
    }
}

fn is_pkg_installed(name: &str) -> bool {
    Command::new("pacman")
        .arg("-Q")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn parse_vendor_id(value: &str) -> Option<GpuVendor> {
    let trimmed = value.trim().trim_start_matches("0x");
    match trimmed.to_ascii_lowercase().as_str() {
        "1002" => Some(GpuVendor::Amd),
        "8086" => Some(GpuVendor::Intel),
        "10de" => Some(GpuVendor::Nvidia),
        _ => None,
    }
}

fn is_gpu_line(line: &str) -> bool {
    line.contains("VGA compatible controller")
        || line.contains("3D controller")
        || line.contains("Display controller")
}

fn parse_vendor_from_lspci(line: &str) -> Option<String> {
    for part in line.split('[').skip(1) {
        let candidate = part.split(':').next()?;
        if candidate.len() == 4 && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(candidate.to_ascii_lowercase());
        }
    }
    None
}

fn extend_unique(target: &mut Vec<String>, values: &[&str]) {
    for value in values {
        if !target.iter().any(|existing| existing == value) {
            target.push((*value).to_string());
        }
    }
}

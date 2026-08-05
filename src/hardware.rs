use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub os: String,
    pub arch: String,
    pub chip_name: String,
    pub cpu_cores: usize,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub is_apple_silicon: bool,
    pub acceleration_engine: String,
}

pub struct HardwareManager;

impl HardwareManager {
    pub fn get_info() -> HardwareInfo {
        let mut sys = System::new_all();
        sys.refresh_all();

        let os = System::name().unwrap_or_else(|| std::env::consts::OS.to_string());
        let arch = std::env::consts::ARCH.to_string();

        let cpu_brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Generic CPU".to_string());

        let is_apple_silicon = cfg!(target_os = "macos") && (arch == "aarch64" || cpu_brand.contains("Apple"));

        let chip_name = if is_apple_silicon {
            if cpu_brand.is_empty() || cpu_brand == "Generic CPU" {
                "Apple Silicon (M-Series / NPU)".to_string()
            } else {
                cpu_brand
            }
        } else {
            cpu_brand
        };

        let acceleration = if is_apple_silicon {
            "Apple Silicon NEON / Accelerate SIMD + Rayon Multi-Core".to_string()
        } else {
            "Multi-Threaded Rayon Parallel SIMD Engine".to_string()
        };

        let cpu_cores = sys.cpus().len().max(1);
        let total_ram_mb = sys.total_memory() / (1024 * 1024);
        let available_ram_mb = sys.available_memory() / (1024 * 1024);

        HardwareInfo {
            os,
            arch,
            chip_name,
            cpu_cores,
            total_ram_mb,
            available_ram_mb,
            is_apple_silicon,
            acceleration_engine: acceleration,
        }
    }
}


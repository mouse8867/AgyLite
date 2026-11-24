//! Launch Antigravity with the proxy injected into its process tree only.

use crate::config::Config;
use crate::detect;
use std::process::Command;
use sysinfo::{ProcessesToUpdate, System};

/// Kill residual Antigravity / language server processes so the next launch
/// is a fresh process that actually inherits the proxy variables.
pub fn kill_residual() -> usize {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let mut killed = 0;
    for proc in sys.processes().values() {
        let name = proc.name().to_string_lossy().to_lowercase();
        if name.contains("antigravity") || name.contains("language_server") {
            if proc.kill() {
                killed += 1;
            }
        }
    }
    killed
}

pub fn launch(cfg: &Config) -> Result<String, String> {
    let exe = cfg
        .agy_path
        .clone()
        .ok_or_else(|| "尚未找到 Antigravity 路径".to_string())?;
    let port = cfg
        .proxy_port
        .ok_or_else(|| "尚未检测到可用代理端口".to_string())?;

    if cfg.kill_residual {
        let n = kill_residual();
        if n > 0 {
            std::thread::sleep(std::time::Duration::from_millis(600));
        }
        let _ = detect::find_processes("antigravity"); // warm cache, ignore
    }

    let proxy = format!("http://{}:{}", cfg.proxy_host, port);
    Command::new(&exe)
        .env("HTTP_PROXY", &proxy)
        .env("HTTPS_PROXY", &proxy)
        .env("http_proxy", &proxy)
        .env("https_proxy", &proxy)
        .env("NO_PROXY", "localhost,127.0.0.1,::1")
        .env("no_proxy", "localhost,127.0.0.1,::1")
        .spawn()
        .map_err(|e| format!("启动失败：{e}"))?;
    Ok(format!("已通过代理 {proxy} 启动 Antigravity"))
}

/// Start-with-Windows toggle via the current-user Run key (no admin).
#[cfg(windows)]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run")
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value("AgyLite", &format!("\"{}\"", exe.display()))
            .map_err(|e| e.to_string())?;
    } else {
        let _ = key.delete_value("AgyLite");
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Err("仅 Windows 支持开机自启".into())
}

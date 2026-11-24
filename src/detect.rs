//! Detection: Antigravity install path, local proxy ports and their owners.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;
use sysinfo::{ProcessesToUpdate, System};

pub const CANDIDATE_PORTS: &[u16] = &[
    10808, 10809, // v2rayN
    7890, 7897, // Clash / Clash Verge / Mihomo
    2080,  // NekoBox / Nekoray
    1080,  // Shadowsocks / sing-box
    1089,  // Qv2ray
    12334, // Hiddify
    20171, 20172,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PortKind {
    /// HTTP proxy request to Google's 204 endpoint succeeded.
    HttpWorking,
    /// Port is open and speaks, but the upstream node did not answer.
    HttpDeadNode,
    /// Only a SOCKS5 handshake succeeds: a pure SOCKS port.
    SocksOnly,
    /// Nothing useful listening.
    Closed,
}

#[derive(Clone, Debug)]
pub struct PortInfo {
    pub port: u16,
    pub kind: PortKind,
    /// Name of the process that owns the listening port, when known.
    pub owner: Option<String>,
    /// Friendly client name guessed from the owner process.
    pub client: Option<String>,
}

#[cfg_attr(not(windows), allow(dead_code))]
fn proc_name(sys: &System, pid: sysinfo::Pid) -> Option<String> {
    sys.process(pid)
        .map(|p| p.name().to_string_lossy().into_owned())
}

/// All running processes whose name contains `needle` (case-insensitive),
/// as (pid, name, exe path).
pub fn find_processes(needle: &str) -> Vec<(u32, String, Option<PathBuf>)> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let needle = needle.to_lowercase();
    sys.processes()
        .values()
        .filter(|p| {
            p.name()
                .to_string_lossy()
                .to_lowercase()
                .contains(&needle)
        })
        .map(|p| {
            (
                p.pid().as_u32(),
                p.name().to_string_lossy().into_owned(),
                p.exe().map(|e| e.to_path_buf()),
            )
        })
        .collect()
}

/// Antigravity path detection, in priority order (product spec section 4):
/// running process -> registry uninstall -> common paths -> bounded search.
pub fn find_antigravity() -> Option<PathBuf> {
    // 1. Running process.
    for (_, _, exe) in find_processes("antigravity") {
        if let Some(p) = exe {
            if p.exists() {
                return Some(p);
            }
        }
    }
    // 2. Registry uninstall entries (Windows).
    #[cfg(windows)]
    {
        if let Some(p) = find_antigravity_registry() {
            return Some(p);
        }
    }
    // 3. Common install locations.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        roots.push(local.join("Programs").join("Antigravity"));
        roots.push(local.join("Antigravity"));
    }
    if let Some(pf) = std::env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(pf).join("Antigravity"));
    }
    for root in &roots {
        let exe = root.join("Antigravity.exe");
        if exe.exists() {
            return Some(exe);
        }
    }
    // 4. Bounded filename search under those roots (depth <= 3).
    for root in &roots {
        if let Some(p) = search_file(root, "Antigravity.exe", 3) {
            return Some(p);
        }
        if let Some(parent) = root.parent() {
            if let Some(p) = search_file(parent, "Antigravity.exe", 2) {
                return Some(p);
            }
        }
    }
    None
}

fn search_file(dir: &Path, name: &str, depth: u32) -> Option<PathBuf> {
    if depth == 0 || !dir.is_dir() {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path.file_name().map(|n| n == name).unwrap_or(false) {
                return Some(path);
            }
        } else if path.is_dir() {
            if let Some(found) = search_file(&path, name, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(windows)]
fn find_antigravity_registry() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;
    let uninstall = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        if let Ok(root) = RegKey::predef(hive).open_subkey(uninstall) {
            for sub in root.enum_keys().flatten() {
                if let Ok(key) = root.open_subkey(&sub) {
                    let name: String = key.get_value("DisplayName").unwrap_or_default();
                    if name.to_lowercase().contains("antigravity") {
                        let loc: String = key.get_value("InstallLocation").unwrap_or_default();
                        if !loc.is_empty() {
                            let exe = PathBuf::from(&loc).join("Antigravity.exe");
                            if exe.exists() {
                                return Some(exe);
                            }
                        }
                        let icon: String = key.get_value("DisplayIcon").unwrap_or_default();
                        let icon = icon.trim_matches('"').trim_end_matches(",0");
                        if !icon.is_empty() && Path::new(icon).exists() {
                            return Some(PathBuf::from(icon));
                        }
                    }
                }
            }
        }
    }
    None
}

fn tcp_connect(port: u16, timeout_ms: u64) -> Option<TcpStream> {
    let addr = format!("127.0.0.1:{port}")
        .to_socket_addrs()
        .ok()?
        .next()?;
    TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).ok()
}

/// Probe one local port: HTTP-proxy first (via Google's 204 endpoint),
/// then a SOCKS5 handshake.
pub fn probe_port(port: u16) -> PortKind {
    // HTTP proxy attempt.
    if let Some(mut stream) = tcp_connect(port, 900) {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(3500)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(1500)));
        let req = "GET http://www.gstatic.com/generate_204 HTTP/1.1\r\nHost: www.gstatic.com\r\nConnection: close\r\n\r\n";
        if stream.write_all(req.as_bytes()).is_ok() {
            let mut buf = [0u8; 512];
            if let Ok(n) = stream.read(&mut buf) {
                if n > 0 {
                    let head = String::from_utf8_lossy(&buf[..n]);
                    if head.contains("204") || head.contains("200") {
                        return PortKind::HttpWorking;
                    }
                    if head.starts_with("HTTP/") {
                        return PortKind::HttpDeadNode;
                    }
                }
            }
        }
    }
    // SOCKS5 handshake attempt.
    if let Some(mut stream) = tcp_connect(port, 900) {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
        if stream.write_all(&[0x05, 0x01, 0x00]).is_ok() {
            let mut buf = [0u8; 2];
            if stream.read_exact(&mut buf).is_ok() && buf[0] == 0x05 {
                return PortKind::SocksOnly;
            }
        }
    }
    PortKind::Closed
}

/// Map a listening port to its owning process name via `netstat -ano`
/// (a system binary; no PowerShell involved).
#[cfg(windows)]
pub fn port_owner(port: u16) -> Option<String> {
    let out = std::process::Command::new("netstat")
        .args(["-ano", "-p", "TCP"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let suffix = format!(":{port}");
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 5 && cols[1].ends_with(&suffix) && cols[3].eq_ignore_ascii_case("LISTENING") {
            if let Ok(pid) = cols[4].parse::<u32>() {
                let mut sys = System::new();
                sys.refresh_processes(ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]), true);
                return proc_name(&sys, sysinfo::Pid::from_u32(pid));
            }
        }
    }
    None
}

#[cfg(not(windows))]
pub fn port_owner(_port: u16) -> Option<String> {
    None
}

/// Friendly client name guessed from the core process that owns the port.
pub fn guess_client(owner: &str) -> Option<String> {
    let n = owner.to_lowercase();
    let name = if n.contains("verge") || n.contains("mihomo") || n.contains("clash") {
        "Clash / Mihomo 系"
    } else if n.contains("sing-box") || n.contains("xray") || n.contains("v2ray") {
        "v2rayN / sing-box 系"
    } else if n.contains("nekobox") || n.contains("nekoray") || n.contains("nekocore") {
        "NekoBox / Nekoray"
    } else if n.contains("shadowsocks") || n.contains("ss-local") {
        "Shadowsocks"
    } else if n.contains("hiddify") {
        "Hiddify"
    } else if n.contains("qv2ray") {
        "Qv2ray"
    } else {
        return None;
    };
    Some(name.to_string())
}

/// Scan all candidate ports and return the interesting ones.
pub fn scan_ports() -> Vec<PortInfo> {
    CANDIDATE_PORTS
        .iter()
        .map(|&port| {
            let kind = probe_port(port);
            let owner = if kind == PortKind::Closed {
                None
            } else {
                port_owner(port)
            };
            let client = owner.as_deref().and_then(guess_client);
            PortInfo {
                port,
                kind,
                owner,
                client,
            }
        })
        .filter(|p| p.kind != PortKind::Closed)
        .collect()
}

/// Best port to launch with: a working HTTP port wins; otherwise the first
/// SOCKS-only port (caller decides how to present it).
pub fn best_port(infos: &[PortInfo]) -> Option<&PortInfo> {
    infos
        .iter()
        .find(|p| p.kind == PortKind::HttpWorking)
        .or_else(|| infos.iter().find(|p| p.kind == PortKind::HttpDeadNode))
        .or_else(|| infos.iter().find(|p| p.kind == PortKind::SocksOnly))
}

/// Is a TUN adapter / route takeover likely active? Heuristic: a network
/// interface whose name mentions tun/wintun/tap/sing-box on Windows.
pub fn tun_likely_active() -> bool {
    #[cfg(windows)]
    {
        if let Ok(out) = std::process::Command::new("ipconfig").output() {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return text.contains("wintun") || text.contains("tun") && text.contains("sing-box");
        }
        false
    }
    #[cfg(not(windows))]
    {
        false
    }
}

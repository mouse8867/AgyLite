//! Preflight checklist (product spec section 5).

use crate::config::Config;
use crate::detect::{self, PortInfo, PortKind};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Pass,
    Warn,
    Fail,
}

#[derive(Clone, Debug)]
pub struct CheckItem {
    pub status: Status,
    pub title: String,
    pub detail: String,
    pub hint: Option<String>,
}

pub struct PreflightResult {
    pub items: Vec<CheckItem>,
    pub ports: Vec<PortInfo>,
}

pub fn run(cfg: &Config) -> PreflightResult {
    let mut items = Vec::new();

    // OS.
    #[cfg(windows)]
    items.push(CheckItem {
        status: if std::env::consts::ARCH == "x86_64" {
            Status::Pass
        } else {
            Status::Warn
        },
        title: "操作系统".into(),
        detail: format!("Windows · {}", std::env::consts::ARCH),
        hint: None,
    });
    #[cfg(not(windows))]
    items.push(CheckItem {
        status: Status::Warn,
        title: "操作系统".into(),
        detail: "非 Windows（开发构建）".into(),
        hint: Some("AgyLite 正式版面向 Windows 10/11 64 位".into()),
    });

    // Antigravity found?
    match &cfg.agy_path {
        Some(p) if std::path::Path::new(p).exists() => items.push(CheckItem {
            status: Status::Pass,
            title: "Antigravity".into(),
            detail: format!("已找到 · {p}"),
            hint: None,
        }),
        _ => items.push(CheckItem {
            status: Status::Fail,
            title: "Antigravity".into(),
            detail: "未找到 Antigravity.exe".into(),
            hint: Some("请先安装 Antigravity，或在设置的高级区手动选择路径".into()),
        }),
    }

    // Proxy ports.
    let ports = detect::scan_ports();
    let best = detect::best_port(&ports);
    match best {
        Some(info) if info.kind == PortKind::HttpWorking => {
            let who = info
                .client
                .clone()
                .or_else(|| info.owner.clone())
                .unwrap_or_else(|| "本地代理".into());
            items.push(CheckItem {
                status: Status::Pass,
                title: "代理端口".into(),
                detail: format!("{} · 127.0.0.1:{} 可用", who, info.port),
                hint: None,
            });
        }
        Some(info) if info.kind == PortKind::HttpDeadNode => items.push(CheckItem {
            status: Status::Fail,
            title: "代理端口".into(),
            detail: format!("端口 {} 有响应，但节点未连通 Google", info.port),
            hint: Some("代理软件在运行但节点可能已断开，请切换节点后重试".into()),
        }),
        Some(info) if info.kind == PortKind::SocksOnly => items.push(CheckItem {
            status: Status::Warn,
            title: "代理端口".into(),
            detail: format!("端口 {} 是纯 SOCKS 端口", info.port),
            hint: Some("建议在代理软件中开启混合 / HTTP 入口端口，兼容性更好".into()),
        }),
        _ => items.push(CheckItem {
            status: Status::Fail,
            title: "代理端口".into(),
            detail: "未检测到可用的本地代理端口".into(),
            hint: Some("请先启动 v2rayN / Clash Verge 等代理软件（TUN 保持关闭即可）".into()),
        }),
    }

    // TUN state.
    if detect::tun_likely_active() {
        items.push(CheckItem {
            status: Status::Warn,
            title: "TUN 状态".into(),
            detail: "检测到 TUN 可能已开启".into(),
            hint: Some("使用 AgyLite 后可以关掉 TUN，全网速度不再受影响".into()),
        });
    } else {
        items.push(CheckItem {
            status: Status::Pass,
            title: "TUN 状态".into(),
            detail: "未开启 TUN（正是本工具的目标状态）".into(),
            hint: None,
        });
    }

    // Residual processes.
    let residual = detect::find_processes("antigravity");
    if residual.is_empty() {
        items.push(CheckItem {
            status: Status::Pass,
            title: "残留进程".into(),
            detail: "无 Antigravity 残留进程".into(),
            hint: None,
        });
    } else if cfg.kill_residual {
        items.push(CheckItem {
            status: Status::Warn,
            title: "残留进程".into(),
            detail: format!("发现 {} 个残留进程，启动时将自动清理", residual.len()),
            hint: None,
        });
    } else {
        items.push(CheckItem {
            status: Status::Warn,
            title: "残留进程".into(),
            detail: format!("发现 {} 个残留进程且未开启自动清理", residual.len()),
            hint: Some("残留进程会让新启动只是唤起旧窗口，代理注入不生效".into()),
        });
    }

    PreflightResult { items, ports }
}

impl PreflightResult {
    pub fn has_fail(&self) -> bool {
        self.items.iter().any(|i| i.status == Status::Fail)
    }
    pub fn pass_count(&self) -> usize {
        self.items.iter().filter(|i| i.status == Status::Pass).count()
    }
}

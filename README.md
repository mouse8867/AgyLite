# AgyLite

**不开 TUN，也能正常用 Google Antigravity。**

Windows 绿色小工具：把本地代理**只注入 Antigravity 这一棵进程树**——
不开虚拟网卡、不改系统代理、不动注册表网络设置，电脑上其它软件和全网速度完全不受影响。

[English](#english-summary) · [下载](#下载) · [许可证](#license) · [交接文档](docs/HANDOVER.md)

![License](https://img.shields.io/badge/license-MIT-copper.svg)
![Version](https://img.shields.io/badge/version-v2.0.0--alpha.4-copper.svg)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-blue.svg)
![Rust](https://img.shields.io/badge/built_with-Rust%20%2B%20egui-orange.svg)
![Size](https://img.shields.io/badge/exe-5.4%20MB-green.svg)

![状态页](docs/assets/screenshots/01-status.png)

---

## 目录

- [这是做什么的](#这是做什么的)
- [原理：进程级代理注入](#原理进程级代理注入)
- [界面](#界面)
- [功能清单](#功能清单)
- [下载](#下载)
- [使用步骤](#使用步骤)
- [设置项说明](#设置项说明)
- [与 TUN / 系统全局代理的对比](#与-tun--系统全局代理的对比)
- [常见问题 FAQ](#常见问题-faq)
- [隐私与安全](#隐私与安全)
- [性能](#性能)
- [从源码构建](#从源码构建)
- [项目结构](#项目结构)
- [交接文档](docs/HANDOVER.md)
- [版本与升级记录](#版本与升级记录)
- [已知限制](#已知限制)
- [免责声明](#免责声明)
- [License](#license)
- [English summary](#english-summary)

---

## 这是做什么的

Antigravity（Google 的 AI IDE）的部分子进程——尤其是 **Language Server** 和 **Agent**——
**不读 Windows 系统代理**。所以只要你用 Clash / v2rayN 这类本地代理，直连 Google 就会失败。

于是很多人被迫**长期开着 TUN 模式**：全机流量被虚拟网卡接管，资源占用高、国内网速慢、
还容易和别的软件打架。

AgyLite 换了个思路：**不接管全机流量，只给 Antigravity 这一棵树注入代理**。

| | TUN 模式 | AgyLite |
| --- | --- | --- |
| 接管范围 | 全机所有流量 | 只有 Antigravity 这一棵进程树 |
| 系统代理设置 | 通常要改 | **不改** |
| 虚拟网卡 | 要 | **不要** |
| 其它软件的网速 | 受影响 | **完全不受影响** |
| 管理员权限 | 通常要 | **不要** |

---

## 原理：进程级代理注入

AgyLite 在**启动 Antigravity 的那一刻**，以子进程方式拉起它，并给这一棵进程树注入标准的
代理环境变量：

```
AgyLite.exe
   │  1. 找到 Antigravity.exe（运行中进程 → 注册表 → 快捷方式 → 常见路径）
   │  2. 找到本机可用的 HTTP / 混合代理端口（端口归属反查 + 协议探测）
   │  3. 以子进程方式启动，并注入代理环境变量
   ▼
Antigravity.exe
   │  HTTP_PROXY=http://127.0.0.1:10808
   │  HTTPS_PROXY=http://127.0.0.1:10808
   │  NO_PROXY=localhost,127.0.0.1,::1
   ▼
   ├── Antigravity 界面进程 ────┐
   ├── Language Server 进程 ────┼──► 127.0.0.1:10808 ──► 你的代理节点 ──► Google
   └── Agent 子进程 ────────────┘

同一台电脑上的其它软件：完全不受影响（没有 TUN、没有系统代理）
```

注入完成之后，**启动器就可以退出了**，Antigravity 不受影响。
但有一个前提必须记住：

> ⚠️ **下次使用 Antigravity 仍然要经 AgyLite 启动。**
> 直接点 Antigravity 原图标 = 没有代理 = 回到直连失败。

---

## 界面

**状态页**：一眼看清环境是否就绪，点一下就启动。

![状态页](docs/assets/screenshots/01-status.png)

**检测页**：启动前的环境自检清单，每一条不通过都给出「怎么办」。

![检测页](docs/assets/screenshots/02-checks.png)

**设置页**：开机自启、启动后行为、手动覆盖路径与端口、关于与赞助。

![设置页](docs/assets/screenshots/03-settings.png)

---

## 功能清单

- **绿色单文件 exe**：免安装、免管理员、**零运行时依赖**（不需要 .NET / WebView2 / Python），
  精简过的系统也能直接跑。
- **Antigravity 路径全自动查找**：正在运行的进程反查 → 注册表 → 快捷方式 → 常见路径 → 有限深度搜索，
  手动选择只是兜底。
- **多代理软件自动识别**：端口归属反查（Windows TCP 表 → PID → 进程名）+ 协议探测，
  能报出「10808 · v2rayN（sing-box 内核）」「7897 · Clash Verge（Mihomo）」这类结论，
  也支持 SOCKS/HTTP 混合端口。
- **启动前环境自检**：✅ / ⚠️ / ❌ 清单，**每条失败都写清楚怎么修**；只有 ❌ 阻断启动，⚠️ 只提醒。
- **纯 SOCKS 端口不静默失败**：会明确提示「这是纯 SOCKS 端口」，并给出换混合端口或改用
  socks5:// 注入的建议。
- **托盘常驻**：启动成功后收进托盘，右键托盘图标可「显示窗口 / 退出」；双击托盘图标也能唤回窗口。
- **开机自启开关**（用户级，可随时关）：自启的是 **AgyLite 托盘常驻**，不会自动拉起 Antigravity。
- **启动后行为三选一**：收进托盘 / 自动关闭 / 保持窗口。
- **残留进程清理**：启动前可选先清掉上次的 Antigravity 残留进程，避免「唤起旧窗口、代理没生效」。
- **界面零截断**：egui 自适应布局 + DPI 感知，125% / 150% 缩放下文字不截断。

---

## 下载

到 **[Releases 页面](https://github.com/mouse8867/AgyLite/releases)** 下载 @AgyLite.exe@（约 5.4 MB），
每个 Release 都附带同名 @.sha256@ 校验文件：

```powershell
# 校验下载到的 exe
Get-FileHash .\AgyLite.exe -Algorithm SHA256
# 与 AgyLite.exe.sha256 里的值比对
```

系统要求：Windows 10 / 11 64 位（Windows 7 未测试）。不需要管理员权限。

---

## 使用步骤

1. **先打开你的代理软件**（v2rayN / Clash Verge / Mihomo / NekoBox / Shadowsocks / Hiddify 等均可），
   **TUN 保持关闭**。AgyLite 不需要你开 TUN。
2. **双击 AgyLite.exe**。它会自动找到 Antigravity 的安装路径，并自动检测可用的本地代理端口
   （能识别端口属于哪个代理软件）。
3. 看一眼「检测」页的自检结果，**全绿**就点 **启动 Antigravity（免 TUN）**。
4. 启动成功后 AgyLite 按你的设置收进托盘 / 自动关闭 / 保持窗口，**Antigravity 正常联网**。

---

## 设置项说明

| 设置 | 说明 |
| --- | --- |
| **开机自启** | 登录后自动在托盘常驻 AgyLite（不自动打开 Antigravity）。写在当前用户的 @HKCU\...\Run@ 键，**不需要管理员**，随时可关。 |
| **启动前清理 Antigravity 残留进程** | 推荐开。避免旧进程被唤起导致代理没注入。 |
| **启动后：收进托盘 / 自动关闭 / 保持窗口** | 启动成功后 AgyLite 自己的去向。默认「收进托盘」。 |
| **高级：Antigravity 路径** | 自动找不到时手动指定。 |
| **高级：代理端口** | 留空＝自动检测。 |

---

## 与 TUN / 系统全局代理的对比

| 维度 | TUN 模式（虚拟网卡） | 系统全局代理 | **AgyLite** |
| --- | --- | --- | --- |
| 生效范围 | 全机所有程序 | 大部分遵循系统代理的程序 | **仅 Antigravity 进程树** |
| 是否改系统设置 | 需装虚拟网卡 / 路由 | 改系统代理开关 | **什么都不改** |
| 其它软件网速 | 明显受影响 | 视程序而定 | **完全不受影响** |
| 需要管理员 | 通常需要 | 需要 | **不需要** |
| 关闭后残留 | 可能残留虚拟网卡与路由 | 需手动还原 | **无残留** |
| Antigravity 的 Agent 子进程 | 生效 | **常常不生效** | **生效** |

---

## 常见问题 FAQ

**Q：为什么不能直接点 Antigravity 的原图标？**
A：代理变量只在启动那一刻注入。直接点图标启动 = 没有代理 = 直连 Google 失败。**每次都要经 AgyLite 启动。**

**Q：关了 AgyLite，Antigravity 还能用吗？**
A：能。代理变量已经写进 Antigravity 的进程环境里了，注入完成之后启动器就多余了。

**Q：支持哪些代理软件？**
A：任何提供本地 HTTP 或混合（HTTP+SOCKS）端口的客户端都可以。常见默认端口都在候选表里：
v2rayN 10808/10809、Clash / Clash Verge / Mihomo 7890/7897、NekoBox/Nekoray 2080、
Shadowsocks 1080、Qv2ray 1089、Hiddify 12334、sing-box 常见 1080 等。
端口被改过时靠「端口归属反查」兜底。

**Q：只有纯 SOCKS 端口行不行？**
A：可以启动，但 AgyLite 会明确提示「检测到的是纯 SOCKS 端口」。最稳的做法是把代理软件切到
**混合 / HTTP 入口端口**；否则部分组件的兼容性问题可能出在节点侧。

**Q：会不会改注册表 / 装驱动 / 留后台服务？**
A：不会改任何网络相关的注册表键，不装驱动，不装服务。唯一写注册表的地方是「开机自启」开关
（@HKCU\Software\Microsoft\Windows\CurrentVersion\Run\AgyLite@），关掉开关即删除该值。

**Q：会收集我的任何数据吗？**
A：不会。AgyLite 不联网上报、不写日志、不收集统计，唯一持久化的是 exe 同目录下的 @config.json@
（Antigravity 路径、代理端口、几个开关）。

**Q：需要管理员权限吗？**
A：不需要。

**Q：怎么彻底卸载？**
A：删掉 @AgyLite.exe@ 和同目录的 @config.json@；如果开了开机自启，先在「设置」页把它关掉
（关掉会删除 Run 键里的值）。

**Q：精简系统（没有 .NET / WebView2）能跑吗？**
A：能。全部功能由单个原生 exe 提供，不依赖任何运行时。

**Q：支持 macOS / Linux 吗？**
A：目前不支持，目标平台只有 Windows。

---

## 隐私与安全

- ❌ 不开 TUN、**不装虚拟网卡**、不改路由表
- ❌ 不修改系统代理设置
- ❌ 不注入 DLL、不修改 Antigravity 本体
- ❌ 不联网上报、不收集任何数据、不写日志
- ✅ 唯一持久化文件：exe 同目录下的 @config.json@
- ✅ 唯一可能的注册表写入：可选的「开机自启」（@HKCU\...\Run\AgyLite@），可随时关闭
- ✅ 全部源码公开，MIT 协议，可自行审计

---

## 性能

| 指标 | 实测 |
| --- | --- |
| exe 体积 | **5.4 MB**（单文件） |
| 冷启动 | < 0.5 秒 |
| 托盘常驻内存 | ≈ 20 MB 以内 |
| 托盘常驻 CPU | ≈ 0% |
| 额外进程 | **无**（不会拉起 powershell.exe 之类的常驻进程） |

---

## 从源码构建

需要 Rust 稳定版工具链：

```powershell
# MSVC 工具链（Windows 上的默认推荐）
cargo build --release --target x86_64-pc-windows-msvc
# 产物：target\x86_64-pc-windows-msvc\release\agylite.exe
```

或者直接从本仓库的 [Releases](https://github.com/mouse8867/AgyLite/releases) 下载已构建好的成品
（每个 Release 都附 SHA256 校验值）。

### CI 自动构建

CI 已启用，工作流在 [.github/workflows/build.yml](.github/workflows/build.yml)：

- 推送到 `main` 或 tag（`v*`）时，在 `windows-latest` 上用 Rust 稳定版构建 Windows x64 成品；
- 推 tag 时自动把 `AgyLite.exe` 与 `AgyLite.exe.sha256` 挂到对应的 Release；
- 运行日志见 [Actions](https://github.com/mouse8867/AgyLite/actions)。

### 托盘行为回归自测

本项目有一个端到端自测脚本，验证「最小化进托盘 → 显示窗口 → 退出」这条链路：

```powershell
pwsh -File scripts\verify_tray.ps1 -Exe .\AgyLite.exe
```

预期输出：

```
[PASS] 窗口与托盘图标已创建
[PASS] 最小化后收进托盘（窗口不可见）
[PASS] 托盘「显示窗口」能重新拉起窗口 — 菜单项 id=1001
[PASS] 托盘「退出」能结束进程 — 菜单项 id=1003 退出码=0
```

### 其它脚本

| 脚本 | 作用 |
| --- | --- |
| @scripts/verify_tray.ps1@ | 托盘链路端到端回归自测 |
| @scripts/capture_screenshots.ps1@ | 重新生成 README 里的界面截图 |
| @scripts/package.ps1@ | 打包「源码 + 成品」为一个 zip |

---

## 项目结构

```
AgyLite/
├── src/
│   ├── main.rs        # 界面（egui/eframe）、三页布局、托盘联动
│   ├── tray.rs        # 托盘图标、菜单、事件监听线程
│   ├── win.rs         # 直接操作窗口的 Win32 封装（托盘显示/退出）
│   ├── detect.rs      # Antigravity 路径查找、端口归属反查与协议探测
│   ├── preflight.rs   # 启动前环境自检清单
│   ├── launch.rs      # 带代理环境变量启动 Antigravity、开机自启开关
│   ├── config.rs      # config.json 读写
│   └── theme.rs       # 配色与中文字体
├── docs/
│   ├── HANDOVER.md          # 交接文档（单一入口：现状/红线/验证命令）
│   ├── PRODUCT_SPEC.md     # 产品规格
│   ├── UI_DESIGN.md        # UI 设计规范
│   └── assets/screenshots/ # README 截图
├── scripts/            # 自测 / 截图 / 打包脚本
├── assets/             # 应用图标
└── .github/workflows/  # CI：构建 + 自动发 Release（已启用）
```

---

## 版本与升级记录

本项目按 alpha 递增发布，每个 alpha 修掉一个用户实测报出的问题。
**完整的变更记录见 [CHANGELOG.md](CHANGELOG.md)**，历史版本可以在
[Releases](https://github.com/mouse8867/AgyLite/releases) 里逐个下载对比。

| 版本 | 关键变化 |
| --- | --- |
| **v2.0.0-alpha.4** | 修复：最小化到托盘后「显示窗口」点不动、「退出」退不掉（只能任务管理器杀）。根因是 eframe 只在出帧时处理视口命令，而隐藏窗口不再收 WM_PAINT 就不再出帧；改为托盘线程直接用 Win32 操作窗口。另加托盘链路端到端自测脚本。 |
| v2.0.0-alpha.3 | 托盘事件改为独立监听线程处理。 |
| v2.0.0-alpha.2 | 引入真实系统托盘图标与托盘菜单。 |
| v2.0.0-alpha.1 | Rust + egui 重写版本（v1 PowerShell 版退役）。 |

> 想看 alpha.3 → alpha.4 具体改了什么，直接比对本仓库的两个 tag：
> @git diff v2.0.0-alpha.3..v2.0.0-alpha.4@

---

## 已知限制

- 仅支持 Windows（10 / 11 x64）。
- 必须经 AgyLite 启动 Antigravity，直接点原图标不会注入代理。
- 代理端口需要是 HTTP 或混合端口；纯 SOCKS 会明确提示但兼容性可能打折。
- 不做节点管理、订阅导入、多账号——那不是这个工具的定位。

---

## 免责声明

AgyLite 是独立社区工具，与 Google 无隶属关系；Antigravity 是 Google 的产品。
本项目不提供任何代理节点、账号或订阅，只把你自己配置的本地代理注入 Antigravity。
请自行确认你的网络与账号使用符合相关服务条款。

---

## License

[MIT](LICENSE) © 2026 mouse8867

---

## 支持作者

AgyLite 免费、开源、无广告。如果它帮你省去了开 TUN 的麻烦，
而你愿意请作者垫吧半个面包，可以自愿赞赏 **¥1.00**——纯属自愿，不影响任何功能与下载。

| 支付宝 | 微信支付 |
| --- | --- |
| ![支付宝收款码](docs/assets/support/alipay-public.jpg) | ![微信收款码](docs/assets/support/wechat-pay-public.jpg) |

联系作者：mouse8867@gmail.com

---

## English summary

**AgyLite** is a tiny portable Windows launcher that makes
[Google Antigravity](https://antigravity.google/) work behind a local proxy —
**without TUN mode and without touching your system proxy settings.**

Antigravity's Language Server and Agent child processes do not honour the Windows
system proxy, so people are forced to keep TUN mode on all day: the whole machine's
traffic goes through a virtual NIC, resource usage climbs, and unrelated apps slow down.

AgyLite takes a different route: it injects the standard proxy environment variables
(HTTP_PROXY / HTTPS_PROXY) into **the Antigravity process tree only**, at launch time.

- 🪟 Single portable AgyLite.exe (~5.4 MB) — no installer, no admin rights,
  **no runtime dependencies** (no .NET, no WebView2).
- 🔍 Finds Antigravity automatically (running process → registry → shortcut → common paths).
- 🌐 Detects and identifies your local proxy port (v2rayN, Clash Verge, Mihomo, NekoBox,
  sing-box, …) and tells you *which* client owns it.
- ✅ Pre-flight self-check: every failed item comes with a concrete fix.
- 📥 Tray-resident after launch: show the window or quit from the tray menu.
- 🔒 Changes nothing: no virtual NIC, no system proxy, no registry network keys, no telemetry.

**Start Antigravity through AgyLite every time** — the proxy variables are injected
at launch, so launching Antigravity directly will fail to reach Google again.

See FAQ above for details. MIT licensed, no ads, no telemetry.

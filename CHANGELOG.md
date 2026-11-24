# Changelog

本项目按 alpha 递增发布：每个 alpha 修掉一个用户实测报出的问题。
更早版本（alpha.1–alpha.3）的修复记录保存在各自源文件顶部的 BUG 注释里
（`src/tray.rs`、`src/main.rs`）。

## v2.0.0-alpha.4 — 2026-11-24

修掉用户报出的最后一个 bug：**最小化到托盘后，「显示窗口」点不动、「退出」也退不掉**，
只能从任务管理器里结束进程。

### 根因

不是托盘事件没到，而是**事件到了却送不到窗口**：

eframe 只在「画一帧」的时候处理 `egui::ViewportCommand`（显示 / 聚焦 / 关闭），
而一帧的前提是 Windows 给窗口发 `WM_PAINT`。窗口一旦被
`set_visible(false)` 藏起来，就再也不收 `WM_PAINT`，于是**再也不出帧**，
所有排队的视口命令（`Visible(true)`、`Close`）永远没人执行——
托盘菜单里点什么都只是「命令进了队列」，死路一条。

此外还有第二个问题：`AgyApp::update()` 和托盘监听线程**同时**消费同一对全局
托盘事件队列，谁先轮询谁吞掉事件。

### 改动

- 新增 `src/win.rs`：直接用 Win32 操作本进程自己的窗口
  （`ShowWindow` + `SetForegroundWindow` 显示并聚焦，`PostMessage(WM_CLOSE)`
  走正常关闭流程），不再依赖已经停摆的帧循环。这是原生托盘程序（含 Tauri）
  的通行做法。
- `src/tray.rs`：监听线程成为托盘事件的**唯一**消费者；「显示窗口」先经 Win32
  把窗口叫醒，再补发视口命令让 egui/eframe 的窗口状态在下一帧对齐；
  「退出」发 WM_CLOSE，让 winit 收到正常的 `CloseRequested`、eframe 干净退出
  （含 `on_exit` 与状态保存），并加了超时兜底，避免再出现「只能任务管理器杀」。
- `src/main.rs`：移除重复的托盘轮询；恢复窗口后有一段短暂宽限期，
  忽略可能残留的「窗口已最小化」标志，避免窗口刚被拉起又被立刻收回去。
- 新增 `scripts/verify_tray.ps1`：托盘链路端到端回归自测。

### 验证

```powershell
cargo build --release --target x86_64-pc-windows-msvc
pwsh -File scripts/verify_tray.ps1
```

预期输出：

```
[PASS] 窗口与托盘图标已创建
[PASS] 最小化后收进托盘（窗口不可见）
[PASS] 托盘「显示窗口」能重新拉起窗口 — 菜单项 id=1001
[PASS] 托盘「退出」能结束进程 — 菜单项 id=1003 退出码=0
```

对照验证：把 `src/tray.rs` 退回 alpha.3 的写法（只发视口命令、不碰 Win32），
第 3、4 项即 FAIL——与用户报的现象完全一致。

## 未发布（Unreleased）

仓库工程化改动，不影响程序行为：

- **CI 正式启用**：工作流从 `docs/ci/build.yml` 迁到 GitHub 认的标准位置
  `.github/workflows/build.yml`。推送 `main` 或 tag（`v*`）自动在 `windows-latest`
  上构建 Windows x64 成品；推 tag 时自动把 `AgyLite.exe` 与 `AgyLite.exe.sha256`
  挂到对应 Release。
- README 同步更新 CI 说明与项目结构树（不再需要手动复制工作流文件）。
- **截图脱敏 + 缩小**：README 里的界面截图原来直接带着本机绝对路径
  （`C:\Users\<账户名>\AppData\Local\Programs\antigravity\Antigravity.exe`），
  而仓库是公开的，等于把 Windows 账户名发出去。现在：
  - 新增 `scripts/redact_screenshots.ps1`：在指定文字行里定位那串路径，用同字体同色重绘成
    `C:\Users\***\AppData\Local\Programs\antigravity\Antigravity.exe`，随后等比缩到
    600px 宽并转成 JPEG（质量 90）；
  - `scripts/capture_screenshots.ps1` 抓完图自动调用它，原始 PNG 落在
    `docs/assets/screenshots/_raw/`（已 gitignore），仓库里只留脱敏后的 `*.jpg`；
  - 三张图从 775×750 的 PNG（合计 404 KB）缩到 600×581 的 JPEG（合计 167 KB），
    仓库体积和 README 排版都变小，界面文字仍然清晰。
- 新增 `docs/HANDOVER.md`：单一入口的交接文档（现状、阻塞清单、验证命令与期望输出、
  交付物地图、行动清单、红线、术语与环境陷阱）。
- **清理 git 历史**：早期提交（`7419136` 及之后）里的原始截图含本机绝对路径，
  已用 `git filter-branch` 从所有历史提交中剔除并强制推送 `main` 与 `v2.0.0-alpha.4` tag，
  两个 Release 保留（资产不变）。重写后所有提交 SHA 变化，已克隆过旧仓库的人需要
  `git fetch origin && git reset --hard origin/main`。

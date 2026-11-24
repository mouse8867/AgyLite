# AgyLite 交接文档

> **单一入口**：接手这个项目先读这一页；细节按需点进链接。本文档对应版本 **v2.0.0-alpha.4**。

---

## 1. 这是什么

| 项 | 值 |
| --- | --- |
| 仓库 | https://github.com/mouse8867/AgyLite （public，MIT） |
| 最新发布 | [v2.0.0-alpha.4](https://github.com/mouse8867/AgyLite/releases/tag/v2.0.0-alpha.4)（Latest，附 `AgyLite.exe` + `AgyLite.exe.sha256`） |
| 技术栈 | Rust 2021 + egui/eframe 0.31 + winit 0.30 + muda 0.16 + tray-icon 0.20 |
| 目标平台 | Windows 10/11 x64（MSVC 工具链） |
| 功能 | 只给 Antigravity 这一棵进程树注入代理环境变量，不开 TUN、不改系统代理 |

---

## 2. 现状

- **托盘 bug 已修复并有 A/B 证据**：v2.0.0-alpha.4 修掉「最小化进托盘后『显示窗口』点不动、『退出』退不掉」。
  根因是 eframe 只在出帧时处理视口命令，窗口隐藏后收不到 `WM_PAINT` → 不再出帧 → 排队的视口命令永远没人执行；
  修复方案是托盘线程直接用 Win32 操作本进程窗口（`src/win.rs`）。
- **CI 已启用**：工作流在 `.github/workflows/build.yml`，推 `main` / tag(`v*`) 自动构建 Windows x64 成品，推 tag 时自动发 Release。
- **成品与源码均已公开**：两个 Release（alpha.3、alpha.4）都在，二进制以附件形式提供，不进 git 树。
- **当前没有未闭环的 bug**，见第 3 节。

---

## 3. 最新未闭环问题与阻塞清单

| 编号 | 事项 | 状态 |
| --- | --- | --- |
| H-1 | 托盘「显示窗口 / 退出」失效 | ✅ 已修复（alpha.4），验证命令见第 4 节 |
| H-2 | CI 无法写入 `.github/workflows/`（token 缺 `workflow` scope） | ✅ 已解决（改用带 `workflow` scope 的凭据，工作流已迁入正式位置并跑通） |
| H-3 | CI 首次正式运行的结果观察 | ✅ 见第 4.3 节记录的运行结论 |

阻塞清单：**无**。若后续要发新版本，唯一的外部依赖是 GitHub 凭据（见第 8 节），
以及本机 Rust 工具链与 VS2022 Build Tools。

---

## 4. 验证命令与期望输出

### 4.1 构建

```powershell
# 在线（需要 crates.io 可达）
cargo build --release --target x86_64-pc-windows-msvc

# 离线（依赖已在本机 cargo registry 时）
cargo build --release --target x86_64-pc-windows-msvc --offline --manifest-path D:\Personal_Data\desktop\AgyLite-alpha.3\Cargo.toml
```

产物：`target\x86_64-pc-windows-msvc\release\agylite.exe`（Rust 包名是小写 `agylite`，
对外发布时复制为根目录的 `AgyLite.exe`）。

### 4.2 托盘链路端到端自测（最关键的一条）

```powershell
pwsh -File scripts\verify_tray.ps1 -Exe .\AgyLite.exe
```

期望输出（4 行 `[PASS]`，退出码 0）：

```
[PASS] 窗口与托盘图标已创建
[PASS] 最小化后收进托盘（窗口不可见）
[PASS] 托盘「显示窗口」能重新拉起窗口 — 菜单项 id=1001
[PASS] 托盘「退出」能结束进程 — 菜单项 id=1003 退出码=0
```

脚本通过真实 `WM_COMMAND` 消息点击托盘菜单项，不是模拟调用；
把 `src/tray.rs` 退回 alpha.3 的写法后，第 3、4 项会 FAIL——这是 bug 的对照证据。

### 4.3 CI

```powershell
gh run list --repo mouse8867/AgyLite --limit 5
gh run view <run-id> --repo mouse8867/AgyLite --log
```

工作流 `.github/workflows/build.yml`：`windows-latest` → `dtolnay/rust-toolchain@stable`
→ `cargo build --release --target x86_64-pc-windows-msvc` → 计算 SHA256 → 上传 artifact；
推 tag 时用 `softprops/action-gh-release` 把 `AgyLite.exe` + `AgyLite.exe.sha256` 挂到 Release。
非 tag 分支上 `Release` 这一步会被自动跳过（expected）。

首次正式运行结果（均 success）：

- run [#45337816336](https://github.com/mouse8867/AgyLite/actions/runs/45337816336)（提交 `27cf202`）
- run [#45338061008](https://github.com/mouse8867/AgyLite/actions/runs/45338061008)（提交 `d645948`）

两次都产出了 artifact `AgyLite-windows`（exe 与 sha256 一起），已核验 artifact 里的
`AgyLite.exe.sha256` 与 artifact 里 exe 的哈希完全一致。

> **2026-11-24 做过一次 git 历史重写**：早期提交里的原始截图含本机绝对路径，
> 已用 `git filter-branch` 从所有历史提交中剔除并强制推送。因此重写前后的提交 SHA 不同，
> 已经克隆过旧仓库的人需要 `git fetch origin && git reset --hard origin/main`（或重新 clone），
> 两个 tag 也已重建（旧 SHA `7419136` → 新 `0d1e74c`）。
注意：CI 在 GitHub runner 上重建的 exe 与 Release 附件里的 exe **不是同一份字节**
（工具链与路径不同，体积差约 1 KB）。对外分发以 Release 附件为准。

### 4.4 打包（源码 + 成品 zip）

```powershell
pwsh -File scripts\package.ps1
```

产物：`dist\AgyLite-2.0.0-alpha.4-source-and-binary.zip`（`dist/` 已被 .gitignore 排除，需重新生成）。

### 4.5 重新生成 README 截图（自动脱敏）

```powershell
pwsh -File scripts\capture_screenshots.ps1
```

流程：抓原始 PNG → `docs/assets/screenshots/_raw/` → 自动调用
`scripts/redact_screenshots.ps1` 脱敏并缩图 → 产出 `docs/assets/screenshots/0{1,2,3}-*.jpg`（600px 宽）。

**为什么要脱敏**：界面上的 Antigravity 绝对路径含本机 Windows 账户名，
仓库是公开的，直接提交原图等于把 `C:\Users\<账户名>\...` 发出去。
脱敏后路径显示为 `C:\Users\***\AppData\Local\Programs\antigravity\Antigravity.exe`。

单独给已有截图补打码：

```powershell
pwsh -File scripts\redact_screenshots.ps1 -InDir docs\assets\screenshots\_raw -OutDir docs\assets\screenshots
```

若改了窗口尺寸或界面文案，`redact_screenshots.ps1` 里的坐标需要重新标定
（脚本会打印实际检测到的文字范围）。

---

## 5. 交付物地图

```
AgyLite/
├── src/
│   ├── main.rs        # 界面（egui/eframe）、三页布局、托盘联动
│   ├── tray.rs        # 托盘图标、菜单、事件监听线程（托盘事件的唯一消费者）
│   ├── win.rs         # 直接操作窗口的 Win32 封装（托盘显示/退出）
│   ├── detect.rs      # Antigravity 路径查找、端口归属反查与协议探测
│   ├── preflight.rs   # 启动前环境自检清单
│   ├── launch.rs      # 带代理环境变量启动 Antigravity、开机自启开关
│   ├── config.rs      # config.json 读写
│   └── theme.rs       # 配色与中文字体
├── docs/
│   ├── HANDOVER.md          # 本文件：交接入口
│   ├── PRODUCT_SPEC.md      # 产品规格
│   ├── UI_DESIGN.md         # UI 设计规范
│   └── assets/screenshots/  # README 截图（600px JPEG，已脱敏）
├── scripts/
│   ├── verify_tray.ps1      # 托盘链路端到端回归自测
│   ├── capture_screenshots.ps1  # 抓图（自动接脱敏流程）
│   ├── redact_screenshots.ps1   # 截图脱敏 + 缩图
│   └── package.ps1          # 打包「源码 + 成品」zip
├── assets/                   # 应用图标
├── .github/workflows/build.yml  # CI：构建 + 自动发 Release
├── CHANGELOG.md              # 逐版本变更记录
├── README.md                 # 对外说明（中英双语）
└── Cargo.toml / Cargo.lock
```

不在 git 树里、但发布时需要的产物：

- 根目录 `AgyLite.exe`（5,651,456 B，sha256 `5ff7c55877ec3eb9c7c1c22c62ec924c190cfef26b4878c1fad5982a8fd9d2c7`）与 `AgyLite.exe.sha256` —— 被 .gitignore 排除，通过 Release 附件分发。
- `dist/` 下的 zip —— 被 .gitignore 排除，用 `scripts/package.ps1` 随时重建。

---

## 6. 行动清单（按优先级）

**P0**

- 无。当前 alpha.4 是可用状态，托盘链路有回归自测兜底。

**P1**

- 持续观察 CI 运行记录：若出现 `failed`，先看 `gh run view <id> --log`，
  多半是 crates.io 网络抖动（可改用 vendor 或加 retry），不是代码问题。
- 若要发新版本（alpha.5），按第 7 节流程走一遍，并同步更新 `Cargo.toml` 版本号、
  `CHANGELOG.md`、README 版本表与徽章。

**P2**

- 可选：把 CI 产物从「仅上传 artifact」升级为「每次 push 都自动建 Draft Release」。
- 可选：加 Rust 签名与 SBOM，发布链路更完整。

---

## 7. 发版流程（照抄即可）

```powershell
# 1) 改版本号：Cargo.toml 的 version = "2.0.0-alpha.5"

# 2) 构建 + 自测
cargo build --release --target x86_64-pc-windows-msvc
pwsh -File scripts\verify_tray.ps1 -Exe .\AgyLite.exe

# 3) 刷新根目录成品与校验值
Copy-Item target\x86_64-pc-windows-msvc\release\agylite.exe .\AgyLite.exe -Force
(Get-FileHash .\AgyLite.exe -Algorithm SHA256).Hash | Out-File .\AgyLite.exe.sha256 -Encoding ascii

# 4) 打包
pwsh -File scripts\package.ps1

# 5) 提交 + 打 tag（国内网络要重试 push）
git add -A
git commit -m "v2.0.0-alpha.5 · <一句话说明>"
git tag v2.0.0-alpha.5
git push origin main v2.0.0-alpha.5

# 6) CI 会自动在 tag 上构建并发 Release；也可手动补发
gh release create v2.0.0-alpha.5 --repo mouse8867/AgyLite --title "AgyLite v2.0.0-alpha.5" --notes-file notes.md .\AgyLite.exe .\AgyLite.exe.sha256
```

---

## 8. 红线（不要动的地方）

1. **托盘事件队列只能有一个消费者**。`src/tray.rs` 的监听线程是唯一消费者；
   不要在 `src/main.rs` 里再加一次轮询——两个消费者会互相吞事件。
2. **隐藏窗口的恢复/退出必须走 `src/win.rs` 的 Win32 调用**。
   不要改回只发 `ViewportCommand`：eframe 不出帧就永远不执行视口命令（就是 alpha.4 修的那个 bug）。
3. **枚举窗口时必须排除 `tray_icon_app` 辅助窗口**（tray-icon 库创建的顶层窗口），
   否则会误把托盘辅助窗口当成主窗口来显示/关闭。
4. **不要把 `AgyLite.exe` / `AgyLite.exe.sha256` / `dist/` 提交进 git**。
   二进制通过 Release 附件分发；.gitignore 已排除。
5. **不要把原始截图（`docs/assets/screenshots/_raw/*.png`）提交进 git**——
   里面含本机绝对路径（`C:\Users\<账户名>\...`）。仓库里只放脱敏后的 `*.jpg`。
6. **不要把任何令牌、密钥、账号密码写进仓库、提交信息、CI 输出或日志**。
7. **不要删除 `v2.0.0-alpha.3` / `v2.0.0-alpha.4` 两个 tag**，它们是「前后版本升级过」的证据。
8. 改动托盘相关代码后，**必须**跑 `scripts/verify_tray.ps1` 再提交。

---

## 9. 术语与环境陷阱

**代码机制**

- eframe 只在 `run_ui_and_paint`（出帧）里调 `egui_winit::process_viewport_commands`；
  隐藏窗口收不到 `WM_PAINT` → 不出帧 → 视口命令全部滞留。这是托盘 bug 的根因。
- winit 的 `request_redraw` 用 `RDW_INTERNALPAINT`，隐藏窗口同样收不到。
- muda（Windows）菜单项原生 id 从 **1000** 起递增：本项目「显示窗口」= 1001、「退出」= 1003，
  `scripts/verify_tray.ps1` 依赖这两个 id，改菜单顺序会让自测脚本失效。
- muda 的 `MenuId` 是字符串类型；本项目用固定 id 之外还做了回调映射。
- 托盘事件跨线程传递用全局队列；监听线程与 UI 线程共用，所以第 8 节红线 1 很关键。

**本机环境**

- Rust 1.100.0 + VS2022 Build Tools；目标 `x86_64-pc-windows-msvc`；可 `--offline` 构建。
- GitHub 凭据：`gh` CLI 的 Windows 凭据管理器里存的是一枚 classic PAT（含 `repo` + `workflow` scope）。
  取回：`gh auth token`；git push 直接复用它（`credential.https://github.com.helper` 指向 `gh auth git-credential`）。
  **令牌本身永不写进仓库。**
- PowerShell 下给 `gh auth login --with-token` 喂令牌**不要用管道**（会因编码问题得到 HTTP 401），
  要用 .NET Process 显式 UTF-8 无 BOM 写 stdin。
- 国内网络推 GitHub 常出现 "Failed to connect to server" / "Connection was reset"，
  `git push` 必须带重试循环；仓库已设 `http.version HTTP/1.1`。
- 本机删除文件受安全策略限制（`Remove-Item` 会被拦），需用可恢复的回收站脚本。

---

## 10. 相关链接

- 产品规格：`docs/PRODUCT_SPEC.md`
- UI 规范：`docs/UI_DESIGN.md`
- 变更记录：`CHANGELOG.md`
- 对外说明：`README.md`
- Releases：https://github.com/mouse8867/AgyLite/releases
- Actions：https://github.com/mouse8867/AgyLite/actions
- Issue：https://github.com/mouse8867/AgyLite/issues
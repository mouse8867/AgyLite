# AgyLite v2 产品规格（公开发布版）

版本：v0.2 已定版，2026-11-24 Xi 确认（v0.2 修订：技术选型由 .NET 改为 Rust，见技术选型节；含多代理软件识别、液态玻璃 UI 规范引用、收款码公开版定版）
状态：v1.0 已在 Xi 本机实测一次成型（v2rayN 端口 10808 自动检测成功、免 TUN 启动成功）；以下为 v2 发布标准。

## 一句话定位
一个 Windows 绿色小工具：不开 TUN、不改系统代理，只给 Antigravity 一个进程级代理，让它在本地代理环境下正常登录和运行。

目标用户：用 v2rayN / Clash Verge / Mihomo 等本地代理、被 Antigravity 必须开 TUN 困扰的 Windows 用户。

## 技术选型（v0.2 已定：Rust）
背景修订：Xi 否决 .NET 路线——目标用户里大量高度精简的系统根本没有 .NET 环境，框架依赖版跑不起来、自包含版又把运行时打进包里（数十 MB、启动与内存都不轻），与“小而轻”的定位冲突。同理排除依赖 WebView2 的 Tauri（精简系统可能连 WebView2 都没有）。

- **Rust + egui，发布为单文件原生 `AgyLite.exe`，零运行时依赖**
- 理由：静态编译单 exe，不依赖 .NET / WebView2 / 任何脚本宿主；资源占用最低（托盘常驻目标见下方硬指标）；进程枚举、注册表、环境变量注入、托盘都可通过 windows-rs / tray-icon 等成熟 crate 完成；egui 自适应布局 + DPI 缩放正好治 v1 的文字截断，且界面可打磨得比原生控件好看。
- 中文字体：运行时加载系统字体（微软雅黑 msyh），不内嵌整套 CJK 字体，避免 exe 膨胀；加载失败时回退内嵌子集字体。
- Go 作为备选而非首选：Go 单 exe 同样零依赖、构建更快，但 Windows GUI 生态弱（Fyne 非原生观感且体积偏大，Walk 已停止维护），界面打磨和长期维护不如 egui 路线稳。若 Rust 的托盘/中文字体在首轮 spike 验证中翻车，再回头评估 Go。
- **性能硬指标（写进验收）**：exe ≤ 15 MB；冷启动 ≤ 0.5 秒；托盘常驻内存 ≤ 20 MB、CPU 约 0%；运行期间不产生任何 powershell.exe 驻留进程。
- v1 的 PowerShell 冲突问题随重构消失：v1 窗口本身就是一个 powershell.exe 进程在跑 WinForms 消息循环，程序开着就等于一个 PowerShell 会话被占着（Xi 实测期间无法再打开管理员 PowerShell，具体触发点未确诊）；原生 exe 不经过 PowerShell、不受执行策略影响。验收新增一条：AgyLite 托盘常驻时，管理员 PowerShell / Windows Terminal 必须能正常打开。
- 未做代码签名时首次运行仍会有 SmartScreen 提示，README 如实说明，签名列入后续版本。

## v2 必做清单（对应 Xi 提出的 6 点）

### 1. 界面全中文 + 美观 + 自有图标 + 文字不截断
- 视觉方案已定（2026-11-24）：采用 Xi 提供的《高级感 UI 资源地图 · 2026》的液态玻璃设计语言，落地细则见同目录 `AgyLite-v2-UI设计规范.md`（借设计语言不借代码：原样配色改冷色调、只取液态胶囊/悬停光斑/就地展开三种交互、玻璃感用静态绘制伪造以守住资源指标）。
- 默认简体中文界面，预留中/英切换（GitHub 面向的不只是中文用户）。
- 布局改自适应布局 + DPI 感知，窗口可缩放，日志区可拉伸；杜绝 v1 的固定坐标截断（截图中 Proxy host/port 标签、底部 Steps 提示都被截断）。
- 自有图标：.ico 多尺寸（16/32/48/256），用于 exe、窗口、托盘、快捷方式。设计方向：深色底 + 轨道环 + 中心点，呼应 Antigravity 的“反重力/轨道”意象，但不使用 Google/Antigravity 官方 Logo，避免商标问题。
- 主界面只留三件事：状态卡片、一键启动按钮、环境检测结果。高级设置（路径、端口、宿主）折叠进“高级”区。

### 2. 启动后窗口何去何从（必须讲清楚）
事实：代理变量只在启动那一刻注入 Antigravity 进程树，注入完成后启动器即可关闭，Antigravity 不受影响；但 Antigravity 完全退出后，下次必须再经 AgyLite 启动，直接点原图标仍会直连失败。
- 启动成功后弹明确提示：“已启动成功，AgyLite 可以关闭。下次使用 Antigravity 请仍从 AgyLite 启动。”
- 行为三选一（设置项，默认第一种）：启动后自动最小化到托盘 / 5 秒倒计时自动关闭 / 保持窗口。
- 托盘模式：右键菜单含“启动 Antigravity / 环境检测 / 开机自启开关 / 关于 / 退出”，双击托盘图标打开主窗口。

### 3. 开机自启（可开可关）
- 开关一个，默认关，用户主动开启。
- 实现用当前用户启动文件夹快捷方式（绿色、透明、可手动删除），不用管理员权限；不用 HKLM。
- 自启的是 **AgyLite 托盘常驻**，不是直接拉起 Antigravity；是否“登录后自动启动 Antigravity”做成另一个独立开关，默认关。

### 4. Antigravity 路径全自动寻找（手动选择降为兜底）
按优先级逐级检测，命中即停，并把结果缓存到本地 config：
1. 正在运行的 Antigravity 进程 → 取其可执行文件路径（解决装在任何奇怪路径的情况）；
2. 注册表卸载项（HKCU / HKLM Uninstall 中 DisplayName 含 Antigravity）→ InstallLocation；
3. 开始菜单 / 桌面快捷方式 → 解析目标路径；
4. 常见路径枚举：`%LOCALAPPDATA%\Programs\Antigravity`、`%LOCALAPPDATA%\Antigravity`、Program Files 等；
5. 在上述根目录下做有限深度文件名搜索（Antigravity.exe）。
全部失败才显示“手动选择”按钮（放在高级区），并给出引导文案。

### 5. 启动前环境自检（Preflight）
一键检测 + 每次启动前自动跑快速版，结果用 ✅/⚠️/❌ 列表呈现，每条失败给修复提示；只有 ❌ 阻断启动，⚠️ 只提醒：
- 操作系统：Windows 10/11、64 位；
- Antigravity：已找到、版本号可读；
- 代理客户端：常见代理进程在运行（v2rayN / sing-box / xray / clash / mihomo 等，检测不到只警告不阻断）；
- 代理端口：候选端口中存在可监听端口；
- 代理可用性：经该端口访问 Google 204 端点成功（区分“端口开但节点死”）；
- 端口类型提醒：若只有纯 SOCKS 端口响应，给出改用混合/HTTP 端口的提示；
- TUN 状态：检测到 TUN 虚拟网卡/路由接管时提示“当前 TUN 已开启，本工具可工作但没必要，可关闭 TUN”；
- 残留进程：存在 Antigravity 残留进程时提示将先清理（对应默认清理选项）。

### 6. 多代理软件支持与自动识别（不限 v2rayN）
原理上 AgyLite 不绑定任何代理软件：它只需要一个本地 HTTP/混合代理端口，注入的是标准 `HTTP_PROXY/HTTPS_PROXY`，任何提供这种端口的客户端都适用。v2 的检测按三层做，不靠猜：
1. **端口归属反查**：枚举本机监听端口，用 Windows TCP 表把端口映射到 PID 再到进程名，直接报出“10808 · v2rayN（sing-box/xray 内核）”“7897 · Clash Verge（mihomo）”这样的结论，而不是只给一个裸端口号。
2. **候选端口 + 协议探测**：候选表覆盖常见客户端默认端口——v2rayN 10808/10809、Clash / Clash Verge / Mihomo 7890/7897、NekoBox/Nekoray 2080、Shadowsocks 1080、Qv2ray 1089、Hiddify 12334、sing-box 常见 1080 等（用户改过端口时靠第 1 层反查兜底）。对每个开放端口做协议探测：先试 HTTP 代理请求（访问 Google 204），再试 SOCKS5 握手，区分“HTTP/混合端口可用”和“纯 SOCKS 端口”。
3. **纯 SOCKS 的处理**：若只有纯 SOCKS 端口，状态页明确提示并给出两条路——在代理软件里开启混合/HTTP 入口（推荐），或使用 AgyLite 的 `socks5://` 注入模式（部分组件兼容性略差，需在真机验证后决定是否保留该模式）。不静默失败。
- 状态主卡展示识别结果：代理软件名 + 端口 + 协议类型；识别不到软件名但端口可用时照常工作，只显示端口。
- 验收新增：分别用 v2rayN、Clash Verge 各跑一次完整流程；另测一个纯 SOCKS 场景，提示文案须准确。

### 7. 关于信息 + 独立应用 + GitHub 发布
- “关于”页：名称 AgyLite、版本号、一句话说明、作者联系邮箱 **mouse8867@gmail.com**、GitHub 仓库链接（建仓后填）、开源许可证、免责声明（见下）。
- “支持作者”区（关于页内 + README 末尾各一处）：支付宝与微信支付收款码各一张，均为固定 ¥1.00、附言“半个面包垫吧垫吧”，文案口径统一为自愿赞赏、不与任何功能/下载挂钩、不设付费解锁。图片已备好：`repo/docs/assets/support/alipay.jpg`、`repo/docs/assets/support/wechat-pay.jpg`。海外用户后续可再加 GitHub Sponsors，本版不做。
- 形态：单文件 `AgyLite.exe`，VBS/PowerShell 版 v1 降为仓库内的 legacy 参考，不再作为发布物。
- 仓库标配：README（中英双语，含截图、原理图、FAQ、与 TUN/全局模式对比表）、LICENSE（建议 MIT）、CHANGELOG、Issues 模板、GitHub Actions 自动构建 Release、每个 Release 附 SHA256 校验值。
- 免责声明要点：独立社区工具，与 Google 无隶属关系；不提供任何代理节点、账号或订阅；只把用户自己配置的本地代理注入 Antigravity 进程；用户自行确认其网络与账号使用符合相关条款。

## 发布前验收（在真机上逐项过）
- [ ] 全新解压、无 config 时首次启动：路径自动找到、端口自动检测、一次启动成功
- [ ] Antigravity 正在运行时打开 AgyLite：路径能从运行进程反查
- [ ] TUN 关闭 / 开启两种状态下都能启动，且 TUN 开启时有对应提示
- [ ] 代理节点断开时：自检能报出“端口开但节点不可用”，而不是笼统失败
- [ ] 启动后关闭 AgyLite：Antigravity 继续正常工作；从原图标启动则回到直连失败（验证注入模型）
- [ ] 开机自启开/关各重启一次验证；自启后只有托盘、无弹窗打扰
- [ ] 125% / 150% 显示缩放下界面无截断
- [ ] 杀软与 SmartScreen 首次运行表现记录进 README
- [ ] AgyLite 运行/托盘常驻期间，管理员 PowerShell 与 Windows Terminal 能正常打开（v1 PowerShell 占用问题的回归测试）；任务管理器中无 AgyLite 拉起的 powershell.exe 驻留
- [ ] 在无 .NET、无 WebView2 的精简系统（或虚拟机）上双击可运行；资源占用满足技术选型节的硬指标

## 暂不做（v2 范围外）
多账号、节点管理、订阅导入、代理内核内置（不学 Cage 内置 Mihomo，保持轻）、macOS/Linux 版、自动更新（v2.1 再议）。

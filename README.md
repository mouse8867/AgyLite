# AgyLite

**不开 TUN，也能正常用 Antigravity。**

AgyLite 是一个 Windows 绿色小工具：它不开启 TUN、不修改系统代理、不动注册表网络设置，只在启动 Antigravity 时把你本地代理的地址注入它这一棵进程树——Antigravity 的界面、Language Server 和 Agent 子进程因此都能联网，而你电脑上的其他软件和全网速度完全不受影响。

> AgyLite is a tiny portable Windows launcher that injects your local HTTP proxy into the Antigravity process tree only. No TUN mode, no system-wide proxy changes, no DLLs.

## 为什么需要它

Antigravity 的子进程（Language Server / Agent）不读取 Windows 系统代理，直连 Google 会失败，于是很多用户被迫长期开着 v2rayN / Clash 的 TUN 模式——全机流量被虚拟网卡接管，资源占用高、国内网速变慢。AgyLite 用进程级注入替代 TUN 兜底，就是为了解决这一件事。

## 使用

1. 打开你的代理软件（v2rayN、Clash Verge、Mihomo、NekoBox、Shadowsocks、Hiddify 等均可），**TUN 保持关闭**。
2. 双击 `AgyLite.exe`。它会自动找到 Antigravity 的安装路径，并自动检测可用的本地代理端口（能识别端口属于哪个代理软件）。
3. 看一眼「检测」页的自检结果，全绿就点 **启动 Antigravity（免 TUN）**。
4. 启动成功后 AgyLite 会自动收起，可以随时关闭。下次使用 Antigravity 时请仍从 AgyLite 启动（直接点 Antigravity 原图标不会带代理）。

## 特性

- 绿色单文件 exe：免安装、免管理员、零运行时依赖（不需要 .NET / WebView2），精简系统也能跑
- 路径全自动查找：运行中进程反查 → 注册表 → 快捷方式 → 常见路径，手动选择只是兜底
- 多代理软件识别：端口归属反查 + 协议探测，区分 HTTP/混合端口与纯 SOCKS 端口（纯 SOCKS 会给出明确提示）
- 启动前环境自检：✅/⚠️/❌ 清单，每条失败都给修复办法
- 开机自启开关（用户级，可随时关）、启动后行为三选一（收起 / 关闭 / 保持）
- 不做的事：不提供任何代理节点、账号或订阅；不收集任何数据；不修改 Antigravity 本体

## 从源码构建

`@powershell
cargo build --release --target x86_64-pc-windows-msvc
`@

或直接从本仓库的 GitHub Actions 下载构建产物（Release 附 SHA256 校验值）。

## 支持作者

AgyLite 免费、开源、无广告。如果它帮你省去了开 TUN 的麻烦，而你愿意请作者垫吧半个面包，可以自愿赞赏 **¥1.00**——纯属自愿，不影响任何功能与下载。

| 支付宝 | 微信支付 |
| --- | --- |
| ![支付宝收款码](docs/assets/support/alipay-public.jpg) | ![微信收款码](docs/assets/support/wechat-pay-public.jpg) |

联系作者：mouse8867@gmail.com

## 免责声明

独立社区工具，与 Google 无隶属关系；Antigravity 是 Google 的产品；请自行确认你的网络与账号使用符合相关服务条款。

## License

MIT

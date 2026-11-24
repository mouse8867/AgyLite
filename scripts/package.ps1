<#
.SYNOPSIS
    打包 AgyLite：源码 + 已构建成品，一个 zip 交付。

.DESCRIPTION
    产物：distAgyLite-<version>-source-and-binary.zip

    包内结构：
        AgyLite-<version>\
            src\                     全部源码
            Cargo.toml / Cargo.lock / build.rs
            assets\ docs\ scripts\ .github\ .cargo\
            README.md / CHANGELOG.md / LICENSE
            AgyLite.exe               成品（Windows x64 单文件）
            AgyLite.exe.sha256        成品校验值
            PACKAGE-INFO.md           包内说明（构建 / 验证方法）

    排除：target\（编译中间产物）、config.json、*.log。

.EXAMPLE
    pwsh -File scripts/package.ps1
#>
param(
    [string]$Version = '',
    [string]$OutDir = ''
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent

if (-not $Version) {
    $Version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version = "(.+)"$').Matches[0].Groups[1].Value
}
if (-not $OutDir) { $OutDir = Join-Path $root 'dist' }
$pkgName = "AgyLite-$Version"

$exe = Join-Path $root 'AgyLite.exe'
if (-not (Test-Path $exe)) {
    throw "找不到成品 $exe —— 先构建：cargo build --release --target x86_64-pc-windows-msvc"
}

$staging = Join-Path $OutDir '.staging'
$pkgDir = Join-Path $staging $pkgName
if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
New-Item -ItemType Directory -Path $pkgDir -Force | Out-Null

# 1) 源码与文档（排除 target/、config.json、*.log）
$skip = @('target', 'dist', '.git')
$files = Get-ChildItem -Path $root -Recurse -File | Where-Object {
    $rel = $_.FullName.Substring($root.Length + 1)
    $parts = $rel -split '\\'
    -not ($parts | Where-Object { $skip -contains $_ }) -and
    $_.Name -ne 'config.json' -and $_.Extension -ne '.log'
}
foreach ($f in $files) {
    $rel = $f.FullName.Substring($root.Length + 1)
    $dest = Join-Path $pkgDir $rel
    New-Item -ItemType Directory -Path (Split-Path $dest -Parent) -Force | Out-Null
    Copy-Item $f.FullName $dest -Force
}

# 2) 成品 + 校验值
Copy-Item $exe (Join-Path $pkgDir 'AgyLite.exe') -Force
$hash = (Get-FileHash (Join-Path $pkgDir 'AgyLite.exe') -Algorithm SHA256).Hash.ToLower()
[System.IO.File]::WriteAllText((Join-Path $pkgDir 'AgyLite.exe.sha256'), "$hash  AgyLite.exe" + [Environment]::NewLine)

# 3) 包内说明
$info = @"
# AgyLite $Version — 源码 + 成品

## 包内容

| 路径 | 说明 |
| --- | --- |
| `AgyLite.exe` | Windows x64 单文件成品，免安装、零运行时依赖 |
| `AgyLite.exe.sha256` | 成品校验值（`sha256sum -c AgyLite.exe.sha256`） |
| `src/` | 全部源码（Rust + egui） |
| `Cargo.toml` / `Cargo.lock` / `build.rs` | 依赖锁定与构建脚本 |
| `assets/` `docs/` | 图标、产品规格、UI 规范、收款码 |
| `scripts/` | 托盘自测（`verify_tray.ps1`）、打包脚本（本文件由 `package.ps1` 生成） |
| `.github/` | CI 构建（Release 自动附 SHA256） |

## 构建

```powershell
cargo build --release --target x86_64-pc-windows-msvc
# 产物：target\x86_64-pc-windows-msvc\release\agylite.exe
```

## 验证托盘行为（alpha.4 修复项）

```powershell
pwsh -File scripts\verify_tray.ps1 -Exe .\AgyLite.exe
```

预期：

```
[PASS] 窗口与托盘图标已创建
[PASS] 最小化后收进托盘（窗口不可见）
[PASS] 托盘「显示窗口」能重新拉起窗口 — 菜单项 id=1001
[PASS] 托盘「退出」能结束进程 — 菜单项 id=1003 退出码=0
```

详见 `CHANGELOG.md`。
"@
[System.IO.File]::WriteAllText((Join-Path $pkgDir 'PACKAGE-INFO.md'), $info, (New-Object System.Text.UTF8Encoding($false)))

# 4) 打包
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$zip = Join-Path $OutDir "$pkgName-source-and-binary.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($staging, $zip, [System.IO.Compression.CompressionLevel]::Optimal, $false)

Remove-Item $staging -Recurse -Force

$zi = Get-Item $zip
Write-Host ("打包完成：{0}" -f $zi.FullName)
Write-Host ("  大小：{0} MB（{1} 个文件）" -f [math]::Round($zi.Length/1MB,2), ($files.Count + 3))
Write-Host ("  SHA256：{0}" -f (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower())

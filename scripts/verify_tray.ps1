<#
.SYNOPSIS
    AgyLite 托盘链路自测：最小化进托盘 → 显示窗口 → 退出。

.DESCRIPTION
    端到端验证 alpha.3 的托盘修复：
      1. 标题栏最小化 → 窗口收进托盘（窗口不可见）；
      2. 托盘「显示窗口」→ 窗口重新出现并可见；
      3. 托盘「退出」→ 进程自行结束（不需要任务管理器）。

    「点击托盘菜单」是通过向 tray-icon 的辅助窗口投递 WM_COMMAND 模拟的，
    与用户真实点击产生的消息完全一致（muda 的菜单子类就是靠它分发事件的）。

    用法：
        pwsh -File scripts/verify_tray.ps1
        pwsh -File scripts/verify_tray.ps1 -Exe .\AgyLite.exe
#>
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\agylite.exe')
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $Exe)) {
    throw "找不到可执行文件：$Exe（先 cargo build --release --target x86_64-pc-windows-msvc）"
}
$Exe = (Resolve-Path $Exe).Path

Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class Win {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);

    public static List<IntPtr> TopLevel(uint pid) {
        var list = new List<IntPtr>();
        EnumWindows(delegate(IntPtr h, IntPtr l) {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid) list.Add(h);
            return true;
        }, IntPtr.Zero);
        return list;
    }
    public static string Text(IntPtr h) { var sb = new StringBuilder(256); GetWindowTextW(h, sb, 256); return sb.ToString(); }
    public static string Cls(IntPtr h) { var sb = new StringBuilder(256); GetClassNameW(h, sb, 256); return sb.ToString(); }
}
"@

$WM_COMMAND    = 0x0111
$WM_SYSCOMMAND = 0x0112
$SC_MINIMIZE   = 0xF020
$SW_HIDE       = 0

$script:Failures = 0

function Step($name, $ok, $detail) {
    $mark = if ($ok) { 'PASS' } else { 'FAIL'; $script:Failures++ }
    Write-Host ("[{0}] {1}{2}" -f $mark, $name, $(if ($detail) { " — $detail" } else { '' }))
}

function Get-AppWindows([int]$ProcId) {
    $main = [IntPtr]::Zero
    $tray = [IntPtr]::Zero
    foreach ($h in [Win]::TopLevel([uint32]$ProcId)) {
        if ([Win]::Cls($h) -eq 'tray_icon_app') { $tray = $h; continue }
        if ([Win]::Text($h) -like 'AgyLite*') { $main = $h }
    }
    @{ Main = $main; Tray = $tray }
}

Write-Host "AgyLite 托盘自测：$Exe"
$proc = Start-Process -FilePath $Exe -PassThru
try {
    # 等窗口和托盘图标出现
    $wins = @{ Main = [IntPtr]::Zero; Tray = [IntPtr]::Zero }
    for ($i = 0; $i -lt 60; $i++) {
        Start-Sleep -Milliseconds 250
        if ($proc.HasExited) { throw "程序启动后立刻退出了（退出码 $($proc.ExitCode)）" }
        $wins = Get-AppWindows $proc.Id
        if ($wins.Main -ne [IntPtr]::Zero -and $wins.Tray -ne [IntPtr]::Zero) { break }
    }
    Step '窗口与托盘图标已创建' ($wins.Main -ne [IntPtr]::Zero -and $wins.Tray -ne [IntPtr]::Zero) "main=$($wins.Main) tray=$($wins.Tray)"
    if ($wins.Main -eq [IntPtr]::Zero -or $wins.Tray -eq [IntPtr]::Zero) { throw '没有找到窗口/托盘图标' }

    # 1) 标题栏最小化 → 收进托盘
    [void][Win]::PostMessageW($wins.Main, $WM_SYSCOMMAND, [IntPtr]$SC_MINIMIZE, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 1200
    $hidden = -not [Win]::IsWindowVisible($wins.Main)
    Step '最小化后收进托盘（窗口不可见）' $hidden ''

    # 2) 找出「显示窗口」并触发它
    $showId = 0
    for ($id = 1000; $id -lt 1012; $id++) {
        if (-not [Win]::IsWindowVisible($wins.Main)) {
            [void][Win]::PostMessageW($wins.Tray, $WM_COMMAND, [IntPtr]$id, [IntPtr]::Zero)
            Start-Sleep -Milliseconds 500
            if ([Win]::IsWindowVisible($wins.Main)) { $showId = $id; break }
        }
    }
    Step '托盘「显示窗口」能重新拉起窗口' ($showId -ne 0) "菜单项 id=$showId"

    # 3) 再收进托盘，然后触发「退出」
    if ($showId -ne 0) {
        [void][Win]::PostMessageW($wins.Main, $WM_SYSCOMMAND, [IntPtr]$SC_MINIMIZE, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 1200
    }
    $quitId = 0
    for ($id = 1000; $id -lt 1012; $id++) {
        if ($proc.HasExited) { break }
        [void][Win]::PostMessageW($wins.Tray, $WM_COMMAND, [IntPtr]$id, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 400
        if ($proc.HasExited) { $quitId = $id; break }
    }
    $exited = $proc.WaitForExit(5000)
    Step '托盘「退出」能结束进程' $exited "菜单项 id=$quitId 退出码=$(if ($exited) { $proc.ExitCode } else { 'n/a' })"
}
finally {
    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
}

if ($script:Failures -gt 0) {
    Write-Host ""
    Write-Host "自测失败：$script:Failures 项未通过" -ForegroundColor Red
    exit 1
}
Write-Host ""
Write-Host "自测全部通过" -ForegroundColor Green

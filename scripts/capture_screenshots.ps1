<#
.SYNOPSIS
    给 README 抓取界面截图（真实运行 AgyLite，截「状态 / 检测 / 设置」三页）。

.DESCRIPTION
    启动成品 → 等窗口 → 置顶并移到屏幕左上 → 等环境检测跑完 → 逐页点击
    顶部分段控件切页 → 抓图。原始截图先落在 docs\assets\screenshots\_raw\，
    再由 scripts\redact_screenshots.ps1 统一脱敏（遮掉界面上的本机绝对路径）
    并缩成 600px JPEG，输出到 docs\assets\screenshots\。

    关键点：脚本必须先 SetProcessDPIAware()，否则 Windows 会把窗口坐标
    按 96 DPI 虚拟化，截出来的是错位的残图（踩过一次）。

.EXAMPLE
    pwsh -File scripts\capture_screenshots.ps1
#>
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\agylite.exe'),
    [string]$OutDir = (Join-Path $PSScriptRoot '..\docs\assets\screenshots'),
    [int]$SettleSeconds = 9
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Exe)) { throw "找不到 $Exe" }
$Exe = (Resolve-Path $Exe).Path
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$OutDir = (Resolve-Path $OutDir).Path
$rawDir = Join-Path $OutDir '_raw'
New-Item -ItemType Directory -Path $rawDir -Force | Out-Null

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class Cap {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }

    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr p);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);

    public static IntPtr Find(uint pid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate(IntPtr h, IntPtr l) {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid) { var sb = new StringBuilder(256); GetWindowTextW(h, sb, 256);
                if (sb.ToString().StartsWith("AgyLite")) { found = h; return false; } }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static void Activate(IntPtr h) {
        uint fgThread = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        uint cur = GetCurrentThreadId();
        bool attached = fgThread != 0 && fgThread != cur && AttachThreadInput(cur, fgThread, true);
        ShowWindow(h, 9); // SW_RESTORE
        SetForegroundWindow(h);
        if (attached) AttachThreadInput(cur, fgThread, false);
    }
    public static void Click(int x, int y) {
        SetCursorPos(x, y);
        System.Threading.Thread.Sleep(150);
        mouse_event(0x0002, 0, 0, 0, IntPtr.Zero);
        System.Threading.Thread.Sleep(70);
        mouse_event(0x0004, 0, 0, 0, IntPtr.Zero);
    }
}
"@

# 必须在任何窗口坐标调用之前声明 DPI 感知，否则坐标被 96 DPI 虚拟化
[void][Cap]::SetProcessDPIAware()

function Save-Shot($hwnd, $path) {
    $r = New-Object Cap+RECT
    [void][Cap]::GetClientRect($hwnd, [ref]$r)
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top

    # 首选 PrintWindow：直接取窗口内容，不受别的窗口遮挡
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    [void][Cap]::PrintWindow($hwnd, $hdc, 2)   # PW_RENDERFULLCONTENT
    $g.ReleaseHdc($hdc)
    $g.Dispose()

    $sum = 0
    for ($x = 0; $x -lt $w; $x += 40) {
        for ($y = 0; $y -lt $h; $y += 40) {
            $sum += $bmp.GetPixel($x, $y).R + $bmp.GetPixel($x, $y).G + $bmp.GetPixel($x, $y).B
        }
    }
    $avg = $sum / [math]::Max(1, ([math]::Ceiling($w / 40) * [math]::Ceiling($h / 40) * 3))

    if ($avg -lt 12) {
        # PrintWindow 拿到黑图（GPU 窗口常见）→ 退回屏幕区域截取
        $bmp.Dispose()
        $p = New-Object Cap+POINT
        [void][Cap]::ClientToScreen($hwnd, [ref]$p)
        $bmp = New-Object System.Drawing.Bitmap($w, $h)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($p.X, $p.Y, 0, 0, (New-Object System.Drawing.Size($w, $h)))
        $g.Dispose()
        $how = 'screen'
    } else {
        $how = 'printwindow'
    }

    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host ("  saved {0}  ({1}x{2}, {3})" -f (Split-Path $path -Leaf), $w, $h, $how)
}

$proc = Start-Process -FilePath $Exe -PassThru
try {
    $hwnd = [IntPtr]::Zero
    for ($i = 0; $i -lt 60; $i++) {
        Start-Sleep -Milliseconds 250
        if ($proc.HasExited) { throw '程序意外退出' }
        $hwnd = [Cap]::Find([uint32]$proc.Id)
        if ($hwnd -ne [IntPtr]::Zero) { break }
    }
    if ($hwnd -eq [IntPtr]::Zero) { throw '没有找到主窗口' }

    # 置顶并移到屏幕左上，避免被别的窗口挡住
    [void][Cap]::SetWindowPos($hwnd, [IntPtr](-1), 40, 40, 0, 0, 0x0001 -bor 0x0040) # NOSIZE|SHOWWINDOW, HWND_TOPMOST
    [Cap]::Activate($hwnd)
    Start-Sleep -Seconds $SettleSeconds   # 等环境检测跑完，状态页才是有内容的

    $r = New-Object Cap+RECT
    [void][Cap]::GetClientRect($hwnd, [ref]$r)
    $cw = $r.Right - $r.Left
    $ch = $r.Bottom - $r.Top
    $scale = [Cap]::GetDpiForWindow($hwnd) / 96.0
    $innerPts = $cw / $scale
    Write-Host ("client {0}x{1} px, dpi scale {2}, inner {3} pt" -f $cw, $ch, $scale, $innerPts)

    # egui 布局（逻辑点）：外边距 8 + 间距 6 + 品牌行 34 + 间距 10 = 58 → 分段控件顶
    $segTopPts = 58.0
    $segH = 36.0
    $margin = 8.0
    $segWPts = ($innerPts - 2 * $margin) / 3.0

    $origin = New-Object Cap+POINT
    [void][Cap]::ClientToScreen($hwnd, [ref]$origin)

    $pages = @(
        @{ i = 0; name = '01-status' },
        @{ i = 1; name = '02-checks' },
        @{ i = 2; name = '03-settings' }
    )
    foreach ($pg in $pages) {
        $x = [int]($origin.X + ($margin + $segWPts * ($pg.i + 0.5)) * $scale)
        $y = [int]($origin.Y + ($segTopPts + $segH / 2) * $scale)
        [Cap]::Click($x, $y)
        Start-Sleep -Milliseconds 900
        Save-Shot $hwnd (Join-Path $rawDir ("$($pg.name).png"))
    }

    # 脱敏 + 缩图：界面上的 Antigravity 绝对路径含本机账户名，公开仓库里不能留
    $redact = Join-Path $PSScriptRoot 'redact_screenshots.ps1'
    if (Test-Path $redact) {
        Write-Host '脱敏并缩图...'
        & $redact -InDir $rawDir -OutDir $OutDir
    } else {
        Write-Warning "找不到 $redact，跳过脱敏：截图里会包含本机绝对路径，不要直接提交"
    }
}
finally {
    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
}

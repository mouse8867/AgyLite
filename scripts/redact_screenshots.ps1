<#
.SYNOPSIS
    对界面截图做「隐私脱敏 + 缩图」：把含本机用户名的绝对路径替换掉，再缩到固定宽度并转 JPEG。

.DESCRIPTION
    为什么需要这一步：
    1) 抓图会直接拍到界面上的 Antigravity 绝对路径（C:\Users\<你的账户>\...），
       README 是公开仓库，这等于把本机账户名和目录结构发出去；
    2) 原图 775x750 的 PNG 单张 120KB+，三张近 400KB，README 里排版也偏大。

    处理动作（每张图）：
    - 在指定文本行里找出最长的一段亮色文字（即那串绝对路径），把整段盖掉；
    - 用同一字体（Microsoft YaHei，与应用内嵌字体一致）、同一前景色，重绘一条打码路径
      C:\Users\***\AppData\Local\Programs\antigravity\Antigravity.exe；
    - 等比缩到 -Width 像素宽，存成 JPEG（质量 -Quality）。

    坐标是按当前 775x750 窗口布局标定的（见 $specs）。若改了窗口尺寸或界面文案，
    脚本会打印实际检测到的文字范围，按提示重新标定即可。

.EXAMPLE
    pwsh -File scripts\redact_screenshots.ps1 -InDir docs\assets\screenshots\_raw -OutDir docs\assets\screenshots -DeleteRaw
#>
param(
    [Parameter(Mandatory = $true)] [string]$InDir,
    [Parameter(Mandatory = $true)] [string]$OutDir,
    [int]$Width = 600,
    [int]$Quality = 90,
    [switch]$DeleteRaw
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$InDir = (Resolve-Path $InDir).Path
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }
$OutDir = (Resolve-Path $OutDir).Path

# 打码后的路径：以真实安装路径为模板，把用户目录换成 C:\Users\***
$realPath = Join-Path $env:LOCALAPPDATA 'Programs\antigravity\Antigravity.exe'
if ($env:USERPROFILE -and $realPath.StartsWith($env:USERPROFILE, [System.StringComparison]::OrdinalIgnoreCase)) {
    $maskedPath = $realPath.Replace($env:USERPROFILE, 'C:\Users\***')
} else {
    $maskedPath = 'C:\Users\***\AppData\Local\Programs\antigravity\Antigravity.exe'
}
Write-Host ('脱敏目标路径: ' + $maskedPath)

# 每张图的文字行标定：BandY = 该行文字的纵向范围；TextX = 路径文字起点；
# Bg = 该行的背景色；Top/Bottom = 原始文字的上下沿（用于把重绘文字对齐）
$specs = @(
    @{ File = '01-status.png';  BandY = @(240, 280); TextX = 217; Bg = @(27, 27, 25);  Top = 252; Bottom = 267 },
    @{ File = '02-checks.png';  BandY = @(310, 350); TextX = 204; Bg = @(47, 62, 81);  Top = 324; Bottom = 340 },
    @{ File = '03-settings.png'; Redact = $false }
)

function Measure-Bright($bmp, $x0, $x1, $y0, $y1, $lum) {
    $top = -1; $bot = -1; $left = -1; $right = -1
    for ($y = $y0; $y -le $y1; $y++) {
        for ($x = $x0; $x -le $x1; $x++) {
            $p = $bmp.GetPixel($x, $y)
            $l = 0.299 * $p.R + 0.587 * $p.G + 0.114 * $p.B
            if ($l -gt $lum) {
                if ($top -lt 0) { $top = $y }; $bot = $y
                if ($left -lt 0 -or $x -lt $left) { $left = $x }
                if ($x -gt $right) { $right = $x }
            }
        }
    }
    return @{ Top = $top; Bottom = $bot; Left = $left; Right = $right }
}

function Longest-Run($bmp, $y0, $y1, $lum) {
    # 先收集「亮像素列」的原始碎片，再把间距 <= 7px 的碎片合并（字母之间的字距），
    # 最后取最长的一段——那才是整串路径文字。
    $raw = New-Object System.Collections.Generic.List[object]
    $cur = -1
    for ($x = 0; $x -lt $bmp.Width; $x++) {
        $has = $false
        for ($y = $y0; $y -le $y1; $y++) {
            $p = $bmp.GetPixel($x, $y)
            $l = 0.299 * $p.R + 0.587 * $p.G + 0.114 * $p.B
            if ($l -gt $lum) { $has = $true; break }
        }
        if ($has) { if ($cur -lt 0) { $cur = $x } }
        elseif ($cur -ge 0) { $raw.Add([pscustomobject]@{ S = $cur; E = $x - 1 }); $cur = -1 }
    }
    if ($cur -ge 0) { $raw.Add([pscustomobject]@{ S = $cur; E = $bmp.Width - 1 }) }

    $merged = New-Object System.Collections.Generic.List[object]
    foreach ($r in $raw) {
        if ($merged.Count -gt 0 -and ($r.S - $merged[$merged.Count - 1].E) -le 8) {
            $merged[$merged.Count - 1].E = $r.E
        }
        else { $merged.Add([pscustomobject]@{ S = $r.S; E = $r.E }) }
    }

    $best = $null
    foreach ($m in $merged) {
        $len = $m.E - $m.S + 1
        if ($null -eq $best -or $len -gt ($best.E - $best.S + 1)) { $best = $m }
    }
    if ($null -eq $best) { return @{ S = -1; E = -1; Len = 0 } }
    return @{ S = $best.S; E = $best.E; Len = $best.E - $best.S + 1 }
}

function Fill-Rect($bmp, $x0, $x1, $y0, $y1, $bg) {
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $br = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb($bg[0], $bg[1], $bg[2]))
    $g.FillRectangle($br, $x0, $y0, ($x1 - $x0 + 1), ($y1 - $y0 + 1))
    $g.Dispose(); $br.Dispose()
}

function Draw-Text($bmp, $text, $tx, $ty, $size, $fg) {
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $font = New-Object System.Drawing.Font('Microsoft YaHei', $size, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
    $br = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb($fg[0], $fg[1], $fg[2]))
    $g.DrawString($text, $font, $br, [float]$tx, [float]$ty)
    $g.Dispose(); $font.Dispose(); $br.Dispose()
}

$fg = @(203, 213, 225)   # 路径文字的原始颜色

foreach ($spec in $specs) {
    $src = Join-Path $InDir $spec.File
    if (-not (Test-Path $src)) { Write-Host ('跳过 ' + $spec.File + '（未找到原始截图）'); continue }

    $orig = [System.Drawing.Bitmap]::FromFile($src)
    $work = $null
    if (-not $spec.ContainsKey('Redact') -or $spec.Redact) {
    $run = Longest-Run $orig $spec.BandY[0] $spec.BandY[1] 140
    if ($run.Len -lt 100) { Write-Host ('警告: ' + $spec.File + ' 未在该行找到路径文字，需重新标定'); $orig.Dispose(); continue }
    Write-Host ('  ' + $spec.File + ' 路径文字范围 x=' + $run.S + '..' + $run.E)

    # 选一个让重绘高度最接近原文字的字号
    $bestSize = 15; $bestDelta = 999
    foreach ($size in 13, 14, 15, 16, 17) {
        $test = New-Object System.Drawing.Bitmap $orig
        Fill-Rect $test $spec.TextX ($run.E + 2) ($spec.Top - 3) ($spec.Bottom + 3) $spec.Bg
        Draw-Text $test $maskedPath $spec.TextX $spec.BandY[0] $size $fg
        $m = Measure-Bright $test $spec.TextX ($run.E + 2) ($spec.Top - 3) ($spec.Bottom + 3) 120
        $h = $m.Bottom - $m.Top + 1
        $d = [math]::Abs($h - ($spec.Bottom - $spec.Top + 1))
        if ($d -lt $bestDelta) { $bestDelta = $d; $bestSize = $size }
        $test.Dispose()
    }

    # 正式重绘，并把文字纵向对齐到原文字位置
    $work = New-Object System.Drawing.Bitmap $orig
    Fill-Rect $work ($spec.TextX - 2) ($run.E + 2) ($spec.Top - 3) ($spec.Bottom + 3) $spec.Bg
    $ty = $spec.BandY[0]
    for ($k = 0; $k -lt 6; $k++) {
        $probe = New-Object System.Drawing.Bitmap $work
        Draw-Text $probe $maskedPath $spec.TextX $ty $bestSize $fg
        $m = Measure-Bright $probe $spec.TextX ($run.E + 2) ($spec.Top - 3) ($spec.Bottom + 3) 120
        $probe.Dispose()
        if ($m.Top -lt 0) { break }
        $shift = $spec.Top - $m.Top
        if ([math]::Abs($shift) -le 0) { break }
        $ty += $shift
    }
    Draw-Text $work $maskedPath $spec.TextX $ty $bestSize $fg
    $orig.Dispose()
    }
    else {
        Write-Host ('  ' + $spec.File + ' 无路径文字，仅缩图')
        $work = $orig
    }

    # 缩到目标宽度并输出 JPEG
    $h2 = [int][math]::Round($Width * $work.Height / $work.Width)
    if ($work -eq $orig) { $orig = $null }
    $small = New-Object System.Drawing.Bitmap $Width, $h2, ([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $g2 = [System.Drawing.Graphics]::FromImage($small)
    $g2.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g2.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g2.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g2.DrawImage($work, (New-Object System.Drawing.Rectangle 0, 0, $Width, $h2))
    $g2.Dispose()

    $dst = Join-Path $OutDir ([System.IO.Path]::GetFileNameWithoutExtension($spec.File) + '.jpg')
    $enc = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
    $ep = New-Object System.Drawing.Imaging.EncoderParameters 1
    $ep.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), ([long]$Quality)
    $small.Save($dst, $enc, $ep)
    $small.Dispose(); $work.Dispose()

    $kb = [math]::Round((Get-Item $dst).Length / 1KB, 1)
    Write-Host ('  -> ' + (Split-Path $dst -Leaf) + '  ' + $Width + 'x' + $h2 + '  ' + $kb + ' KB')
}

if ($DeleteRaw) {
    Get-ChildItem -File $InDir -Filter '*.png' | ForEach-Object {
        Remove-Item $_.FullName -Force
        Write-Host ('  删除原始截图 ' + $_.Name)
    }
    if (Test-Path $InDir) { Remove-Item $InDir -Recurse -Force -ErrorAction SilentlyContinue }
}
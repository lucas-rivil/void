$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
$icons = Join-Path $root 'apps/desktop/src-tauri/icons'
New-Item -ItemType Directory -Force -Path $icons | Out-Null

function New-Canvas {
    param([int]$Size)
    $bmp = New-Object System.Drawing.Bitmap($Size, $Size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.TextRenderingHint = 'AntiAliasGridFit'
    $g.InterpolationMode = 'HighQualityBicubic'
    return @($bmp, $g)
}

function New-VoidBitmap {
    param([int]$Size)
    $pair = New-Canvas -Size $Size
    $bmp = $pair[0]
    $g = $pair[1]

    $scale = $Size / 256.0
    $margin = [int](14 * $scale)
    $radius = [int](52 * $scale)

    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $rect = New-Object System.Drawing.Rectangle ($margin, $margin, ($Size - 2 * $margin), ($Size - 2 * $margin))
    $path.AddArc($rect.X, $rect.Y, (2 * $radius), (2 * $radius), 180, 90)
    $path.AddArc(($rect.Right - 2 * $radius), $rect.Y, (2 * $radius), (2 * $radius), 270, 90)
    $path.AddArc(($rect.Right - 2 * $radius), ($rect.Bottom - 2 * $radius), (2 * $radius), (2 * $radius), 0, 90)
    $path.AddArc($rect.X, ($rect.Bottom - 2 * $radius), (2 * $radius), (2 * $radius), 90, 90)
    $path.CloseFigure()

    $bg = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(242, 242, 242))
    $g.FillPath($bg, $path)

    $ringPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(20, 20, 20)), ([single](10 * $scale))
    $ring = New-Object System.Drawing.Rectangle ([int](52 * $scale), ([int](52 * $scale)), ([int](152 * $scale)), ([int](152 * $scale)))
    $g.DrawEllipse($ringPen, $ring)

    $fontSize = [int](118 * $scale)
    if ($fontSize -lt 6) { $fontSize = 6 }
    $font = New-Object System.Drawing.Font ('Segoe UI', $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $black = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(15, 15, 15))
    $fmt = New-Object System.Drawing.StringFormat
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center
    $fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
    $text = New-Object System.Drawing.RectangleF (0, ($Size * 0.03), $Size, $Size)
    $g.DrawString('V', $font, $black, $text, $fmt)

    $g.Dispose()
    return $bmp
}

$main = New-VoidBitmap -Size 256
$pngPath = Join-Path $icons 'icon.png'
$main.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)

$sizes = @(256, 128, 64, 48, 32, 24, 16)
$pngs = @{}
foreach ($s in $sizes) {
    if ($s -eq 256) {
        $pngs[$s] = [IO.File]::ReadAllBytes($pngPath)
    }
    else {
        $pair = New-Canvas -Size $s
        $small = $pair[0]
        $gSmall = $pair[1]
        $gSmall.DrawImage($main, 0, 0, $s, $s)
        $gSmall.Dispose()
        $ms = New-Object System.IO.MemoryStream
        $small.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $small.Dispose()
        $pngs[$s] = $ms.ToArray()
    }
}
$main.Dispose()

$ico = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ico)
$bw.Write([uint16]0)
$bw.Write([uint16]1)
$bw.Write([uint16]$sizes.Count)

$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $s = $sizes[$i]
    $dim = if ($s -ge 256) { 0 } else { $s }
    $bw.Write([byte]$dim)
    $bw.Write([byte]$dim)
    $bw.Write([byte]0)
    $bw.Write([byte]0)
    $bw.Write([uint16]1)
    $bw.Write([uint16]32)
    $bw.Write([uint32]$pngs[$s].Length)
    $bw.Write([uint32]$offset)
    $offset += $pngs[$s].Length
}
foreach ($s in $sizes) {
    $bw.Write($pngs[$s])
}
$bw.Flush()
[IO.File]::WriteAllBytes((Join-Path $icons 'icon.ico'), $ico.ToArray())

Write-Host "icone V noir generee (tailles: $($sizes -join ', ')) dans $icons"

<#
/************************************************
* File: uninstall.ps1
* Author: Michal Švrček
*
* NetProbe Windows uninstaller.
*
* ver. 0.6.0
*************************************************/
#>

$ErrorActionPreference = "Stop"

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\NetProbe"
$Binary = Join-Path $InstallDir "nprobe.exe"

Write-Host "NetProbe Windows Uninstaller" -ForegroundColor Cyan

if (Test-Path -LiteralPath $Binary) {
    Remove-Item -LiteralPath $Binary -Force
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")

$Entries = @(
    $UserPath -split ';' | Where-Object {
        -not [string]::IsNullOrWhiteSpace($_) -and
        $_.TrimEnd('\') -ine $InstallDir.TrimEnd('\')
    }
)

[Environment]::SetEnvironmentVariable(
    "Path",
    ($Entries -join ';'),
    "User"
)

Write-Host "[OK] NetProbe uninstalled." -ForegroundColor Green
Write-Host "Saved device profiles were preserved."

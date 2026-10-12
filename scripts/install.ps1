<#
/************************************************
* File: install.ps1
* Author: Michal Švrček
*
* NetProbe Windows installer.
*
* ver. 0.6.0
*************************************************/
#>

[CmdletBinding()]
param(
    [string]$BinaryPath
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

if ([string]::IsNullOrWhiteSpace($BinaryPath)) {
    $LocalBuild = Join-Path $ProjectRoot "target\release\nprobe.exe"
    $ReleaseBinary = Join-Path $ScriptDir "nprobe.exe"

    if (Test-Path -LiteralPath $LocalBuild -PathType Leaf) {
        $BinaryPath = $LocalBuild
    }
    else {
        $BinaryPath = $ReleaseBinary
    }
}

if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
    throw "NetProbe binary not found: $BinaryPath"
}

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\NetProbe"
$Destination = Join-Path $InstallDir "nprobe.exe"

Write-Host "NetProbe Windows Installer" -ForegroundColor Cyan

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

$Source = (Resolve-Path -LiteralPath $BinaryPath).Path

if (-not [string]::Equals(
    $Source,
    $Destination,
    [StringComparison]::OrdinalIgnoreCase
)) {
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")

$Entries = @(
    $UserPath -split ';' | Where-Object {
        -not [string]::IsNullOrWhiteSpace($_)
    }
)

$Exists = @($Entries | Where-Object {
    $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\')
}).Count -gt 0

if (-not $Exists) {
    $Entries += $InstallDir

    [Environment]::SetEnvironmentVariable(
        "Path",
        ($Entries -join ';'),
        "User"
    )

    Write-Host "[OK] Added NetProbe to PATH." -ForegroundColor Green
}

$ConfigDir = Join-Path $env:APPDATA "NetProbe"
New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null

Write-Host ""
Write-Host "[OK] NetProbe installed." -ForegroundColor Green
Write-Host "Binary: $Destination"

& $Destination --version

Write-Host ""
Write-Host "Open a new PowerShell terminal."
Write-Host "Run: nprobe --help"

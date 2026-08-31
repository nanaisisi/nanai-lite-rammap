<#
.SYNOPSIS
Build and package the nanai-lite-rammap app using winapp and MSIX.

.DESCRIPTION
This script supports two modes:
- Debug: build debug binary, create debug identity, and run the app.
- Package: build release binary, create a dist folder, generate certificate, pack MSIX, install cert, and install MSIX.

.EXAMPLE
    .\build-msix.ps1 -Mode Debug

.EXAMPLE
    .\build-msix.ps1 -Mode Package
#>

[CmdletBinding()]
param (
    [ValidateSet('Debug', 'Package')]
    [string]$Mode = 'Debug',

    [switch]$SkipCertInstall,
    [switch]$SkipMsixInstall,
    [switch]$SkipPack,
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $projectRoot
try {
    $exeName = 'nanai-lite-rammap.exe'
    $debugPath = Join-Path $projectRoot 'target\debug\' $exeName
    $releasePath = Join-Path $projectRoot 'target\release\' $exeName
    $distDir = Join-Path $projectRoot 'dist'
    $certPath = Join-Path $projectRoot 'devcert.pfx'
    $msixName = 'nanai-lite-rammap.msix'
    $msixPath = Join-Path $projectRoot $msixName
    $packageName = 'nanai-lite-rammap'
    $manifestPath = Join-Path $projectRoot 'Package.appxmanifest'

    function Build-Debug {
        if (-not $SkipBuild) {
            Write-Host 'Building debug binary...' -ForegroundColor Cyan
            cargo build
        }
        if (-not (Test-Path $debugPath)) {
            throw "Debug binary not found: $debugPath"
        }
    }

    function Build-Release {
        if (-not $SkipBuild) {
            Write-Host 'Building release binary...' -ForegroundColor Cyan
            cargo build --release
        }
        if (-not (Test-Path $releasePath)) {
            throw "Release binary not found: $releasePath"
        }
    }

    function New-DebugIdentity {
        Write-Host 'Creating debug identity...' -ForegroundColor Cyan
        winapp create-debug-identity $debugPath
    }

    function Start-DebugBinary {
        Write-Host 'Running debug binary...' -ForegroundColor Cyan
        & $debugPath
    }

    function New-Dist {
        if (Test-Path $distDir) {
            Remove-Item -Path $distDir -Recurse -Force
        }
        New-Item -ItemType Directory -Path $distDir | Out-Null
        
        Write-Host 'Copying binaries and assets to dist...' -ForegroundColor Cyan
        Copy-Item -Path $releasePath -Destination $distDir -Force
        
        # Copy WinUI / Windows App SDK runtime DLLs and PRI files
        Copy-Item -Path (Join-Path $projectRoot 'target\release\*.dll') -Destination $distDir -Force -ErrorAction SilentlyContinue
        Copy-Item -Path (Join-Path $projectRoot 'target\release\*.pri') -Destination $distDir -Force -ErrorAction SilentlyContinue
        
        # Copy manifest
        if (Test-Path $manifestPath) {
            Copy-Item -Path $manifestPath -Destination $distDir -Force
        } elseif (Test-Path (Join-Path $projectRoot 'appxmanifest.xml')) {
            Copy-Item -Path (Join-Path $projectRoot 'appxmanifest.xml') -Destination $distDir -Force
        }

        # Copy Assets
        if (Test-Path (Join-Path $projectRoot 'Assets')) {
            Copy-Item -Path (Join-Path $projectRoot 'Assets') -Destination $distDir -Recurse -Force
        }
    }

    function New-Cert {
        Write-Host 'Generating development certificate...' -ForegroundColor Cyan
        winapp cert generate --if-exists skip
        if (-not (Test-Path $certPath)) {
            throw "Certificate file not created: $certPath"
        }
    }

    function New-MsixPackage {
        Write-Host 'Packing MSIX...' -ForegroundColor Cyan
        $manifestInDist = if (Test-Path (Join-Path $distDir 'Package.appxmanifest')) {
            Join-Path $distDir 'Package.appxmanifest'
        } else {
            Join-Path $distDir 'appxmanifest.xml'
        }
        winapp package $distDir --manifest $manifestInDist --output $msixPath --cert $certPath
        if (-not (Test-Path $msixPath)) {
            Write-Warning "MSIX package not found at expected path: $msixPath"
        }
    }

    function Install-Cert {
        Write-Host 'Installing certificate (administrative permissions required)...' -ForegroundColor Cyan
        winapp cert install $certPath
    }

    function Install-Msix {
        Write-Host 'Installing MSIX package...' -ForegroundColor Cyan
        $existingPackages = Get-AppxPackage | Where-Object { $_.Name -like "$packageName*" }
        foreach ($pkg in $existingPackages) {
            Write-Host "Removing existing package: $($pkg.PackageFullName)" -ForegroundColor Yellow
            Remove-AppxPackage -Package $pkg.PackageFullName -ErrorAction SilentlyContinue
        }
        Add-AppxPackage -ForceApplicationShutdown -ForceUpdateFromAnyVersion $msixPath
    }

    switch ($Mode) {
        'Debug' {
            Build-Debug
            New-DebugIdentity
            Start-DebugBinary
        }
        'Package' {
            Build-Release
            New-Dist
            New-Cert
            if (-not $SkipPack) {
                New-MsixPackage
            }
            if (-not $SkipCertInstall) {
                Install-Cert
            }
            if (-not $SkipMsixInstall) {
                Install-Msix
            }
        }
    }
}
finally {
    Pop-Location
}

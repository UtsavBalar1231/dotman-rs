# PowerShell installation script for dotman on Windows
# Supports winget, Scoop, Chocolatey, and direct download

param(
    [string]$InstallDir = "$env:USERPROFILE\.local\bin",
    [switch]$ForceDownload,
    [switch]$Help,
    [switch]$Version
)

# Configuration
$RepoUrl = "https://github.com/UtsavBalar1231/dotman-rs"
$BinaryName = "dot.exe"

# Colors for output (if supported)
$Colors = @{
    Red = "Red"
    Green = "Green"
    Yellow = "Yellow"
    Blue = "Blue"
    Cyan = "Cyan"
}

function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White",
        [string]$Prefix = ""
    )
    
    if ($Prefix) {
        Write-Host "[$Prefix] " -ForegroundColor $Color -NoNewline
        Write-Host $Message
    } else {
        Write-Host $Message -ForegroundColor $Color
    }
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput -Message $Message -Color $Colors.Blue -Prefix "INFO"
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput -Message $Message -Color $Colors.Green -Prefix "SUCCESS"
}

function Write-Warning {
    param([string]$Message)
    Write-ColorOutput -Message $Message -Color $Colors.Yellow -Prefix "WARNING"
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput -Message $Message -Color $Colors.Red -Prefix "ERROR"
}

function Show-Help {
    @"
dotman Installation Script for Windows

USAGE:
    install-windows.ps1 [OPTIONS]

OPTIONS:
    -InstallDir <DIR>     Custom installation directory (default: ~/.local/bin)
    -ForceDownload        Force download from GitHub releases
    -Help                 Show this help message
    -Version              Show version information

EXAMPLES:
    .\install-windows.ps1                           # Auto-detect and install
    .\install-windows.ps1 -ForceDownload            # Force GitHub download
    .\install-windows.ps1 -InstallDir "C:\Tools"    # Custom install location

PACKAGE MANAGERS:
    The script will try these methods in order:
    1. winget (Windows Package Manager)
    2. Scoop
    3. Chocolatey
    4. Direct download from GitHub releases

"@ | Write-Host
}

function Show-Version {
    Write-Host "dotman Windows installer v1.0"
}

function Test-Command {
    param([string]$Command)
    try {
        Get-Command $Command -ErrorAction Stop | Out-Null
        return $true
    }
    catch {
        return $false
    }
}

function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "$RepoUrl/releases/latest" -ErrorAction Stop
        return $response.tag_name -replace '^v', ''
    }
    catch {
        Write-Warning "Could not fetch latest version, using default"
        return "0.0.1"
    }
}

function Get-Architecture {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default { 
            Write-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
            exit 1
        }
    }
}

function Install-ViaWinget {
    if (-not (Test-Command "winget")) {
        return $false
    }
    
    Write-Info "Installing via winget..."
    try {
        winget install UtsavBalar1231.dotman
        return $true
    }
    catch {
        Write-Warning "winget installation failed: $_"
        return $false
    }
}

function Install-ViaScoop {
    if (-not (Test-Command "scoop")) {
        return $false
    }
    
    Write-Info "Installing via Scoop..."
    try {
        scoop bucket add utsav https://github.com/UtsavBalar1231/scoop-bucket
        scoop install dotman
        return $true
    }
    catch {
        Write-Warning "Scoop installation failed: $_"
        return $false
    }
}

function Install-ViaChocolatey {
    if (-not (Test-Command "choco")) {
        return $false
    }
    
    Write-Info "Installing via Chocolatey..."
    try {
        choco install dotman
        return $true
    }
    catch {
        Write-Warning "Chocolatey installation failed: $_"
        return $false
    }
}

function Install-FromGitHub {
    $version = Get-LatestVersion
    $arch = Get-Architecture
    $platform = "windows-$arch"
    
    Write-Info "Installing dotman v$version for $platform"
    
    $downloadUrl = "$RepoUrl/releases/download/v$version/dotman-rs-$version-$platform.zip"
    $tempDir = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString()
    $zipPath = "$tempDir\dotman.zip"
    
    try {
        # Create temp directory
        New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
        
        # Download the release
        Write-Info "Downloading from $downloadUrl"
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath
        
        # Extract the archive
        Expand-Archive -Path $zipPath -DestinationPath $tempDir
        
        # Create install directory
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        
        # Install binary
        $sourceBinary = Get-ChildItem -Path $tempDir -Filter $BinaryName -Recurse | Select-Object -First 1
        if ($sourceBinary) {
            $targetPath = Join-Path $InstallDir $BinaryName
            Copy-Item -Path $sourceBinary.FullName -Destination $targetPath -Force
            Write-Success "dotman installed to $targetPath"
        } else {
            Write-Error "Binary not found in downloaded archive"
            return $false
        }
        
        return $true
    }
    catch {
        Write-Error "GitHub installation failed: $_"
        return $false
    }
    finally {
        # Cleanup
        if (Test-Path $tempDir) {
            Remove-Item -Path $tempDir -Recurse -Force
        }
    }
}

function Add-ToPath {
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    
    if ($currentPath -like "*$InstallDir*") {
        Write-Info "Install directory already in PATH"
        return
    }
    
    $newPath = if ($currentPath) { "$InstallDir;$currentPath" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    
    # Update current session PATH
    $env:PATH = "$InstallDir;$env:PATH"
    
    Write-Success "Added $InstallDir to PATH"
    Write-Warning "Please restart your terminal or PowerShell session"
}

function Test-Installation {
    $binaryPath = Join-Path $InstallDir $BinaryName
    
    if (Test-Path $binaryPath) {
        try {
            $output = & $binaryPath --version 2>$null
            if ($LASTEXITCODE -eq 0) {
                $version = $output -split ' ' | Select-Object -Last 1
                Write-Success "Installation verified! dotman $version is ready to use."
                Write-Info "Try: dot --help"
                return $true
            }
        }
        catch { }
    }
    elseif (Test-Command "dot") {
        try {
            $output = dot --version 2>$null
            if ($LASTEXITCODE -eq 0) {
                $version = $output -split ' ' | Select-Object -Last 1
                Write-Success "Installation verified! dotman $version is ready to use."
                Write-Info "Try: dot --help"
                return $true
            }
        }
        catch { }
    }
    
    Write-Error "Installation verification failed. Binary not found or not working."
    return $false
}

function Show-CompletionSetup {
    @"

SHELL COMPLETION SETUP:

Enable PowerShell completions for better usability:

Add to your PowerShell profile:
  Add-Content `$PROFILE 'Invoke-Expression (& dot completion powershell)'

To find your profile location:
  `$PROFILE

If the profile doesn't exist, create it:
  New-Item -ItemType File -Path `$PROFILE -Force

"@ | Write-Host -ForegroundColor Cyan
}

# Handle command line arguments
if ($Help) {
    Show-Help
    exit 0
}

if ($Version) {
    Show-Version
    exit 0
}

# Main installation logic
Write-ColorOutput -Message "dotman Installation Script for Windows" -Color $Colors.Cyan
Write-Info "Target installation directory: $InstallDir"

$installed = $false

if (-not $ForceDownload) {
    # Try package managers first
    if (Install-ViaWinget) {
        $installed = $true
    }
    elseif (Install-ViaScoop) {
        $installed = $true
    }
    elseif (Install-ViaChocolatey) {
        $installed = $true
    }
}

# Fall back to GitHub download
if (-not $installed) {
    if (Install-FromGitHub) {
        Add-ToPath
        $installed = $true
    }
}

if (-not $installed) {
    Write-Error "All installation methods failed"
    exit 1
}

# Verify installation
if (Test-Installation) {
    Show-CompletionSetup
    Write-ColorOutput -Message "🎉 Installation complete!" -Color $Colors.Green
    Write-Info "Get started with: dot init"
} else {
    Write-Error "Installation failed verification"
    exit 1
}
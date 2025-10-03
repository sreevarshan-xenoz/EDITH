# Fix Rust build on Windows

Write-Host "🔧 Fixing Rust build environment..." -ForegroundColor Green

# Option 1: Install Visual Studio Build Tools
Write-Host "Option 1: Install Microsoft C++ Build Tools" -ForegroundColor Yellow
Write-Host "Run: winget install Microsoft.VisualStudio.2022.BuildTools" -ForegroundColor Cyan
Write-Host "Then select 'C++ build tools' workload" -ForegroundColor Gray

Write-Host ""

# Option 2: Use GNU toolchain instead
Write-Host "Option 2: Switch to GNU toolchain (faster)" -ForegroundColor Yellow
Write-Host "This avoids needing Visual Studio entirely" -ForegroundColor Gray

# Add Cargo to PATH permanently
$cargoPath = "$env:USERPROFILE\.cargo\bin"
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")

if ($currentPath -notlike "*$cargoPath*") {
    Write-Host "Adding Cargo to PATH..." -ForegroundColor Blue
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$cargoPath", "User")
    $env:PATH += ";$cargoPath"
    Write-Host "✅ Cargo added to PATH" -ForegroundColor Green
}

# Check if we can switch to GNU toolchain
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    Write-Host "Switching to GNU toolchain..." -ForegroundColor Blue
    rustup toolchain install stable-x86_64-pc-windows-gnu
    rustup default stable-x86_64-pc-windows-gnu
    Write-Host "✅ Switched to GNU toolchain" -ForegroundColor Green
    Write-Host "Now try: cargo build --release" -ForegroundColor Cyan
} else {
    Write-Host "❌ rustup not found in PATH" -ForegroundColor Red
    Write-Host "Restart your terminal and try again" -ForegroundColor Yellow
}
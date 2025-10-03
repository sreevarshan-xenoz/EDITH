# Build script for Windows

Write-Host "🦀 Building LLM Wrapper..." -ForegroundColor Green

# Check if Rust is installed
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Rust not found. Installing..." -ForegroundColor Red
    Write-Host "💡 Download from: https://rustup.rs/" -ForegroundColor Yellow
    Write-Host "Or run: winget install Rustlang.Rustup" -ForegroundColor Yellow
    exit 1
}

# Build the project
Write-Host "🔨 Building release binary..." -ForegroundColor Blue
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Build successful!" -ForegroundColor Green
    Write-Host "📦 Binary location: target/release/llm.exe" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage examples:" -ForegroundColor Yellow
    Write-Host "  ./target/release/llm.exe 'Hello!'" -ForegroundColor Gray
    Write-Host "  ./target/release/llm.exe chat" -ForegroundColor Gray
    Write-Host "  ./target/release/llm.exe list" -ForegroundColor Gray
} else {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}
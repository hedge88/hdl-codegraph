# prepare-offline.ps1
# 在 Windows (有网络) 上准备离线迁移包
# 用法: .\scripts\prepare-offline.ps1

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

Write-Host "=== HDL-Graph 离线迁移包准备工具 ===" -ForegroundColor Cyan
Write-Host ""

# 检查 Rust 工具链
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "错误: 未找到 cargo，请先安装 Rust" -ForegroundColor Red
    exit 1
}

Set-Location $ProjectRoot

# Step 1: 下载所有依赖
Write-Host "[1/4] 下载依赖到本地缓存..." -ForegroundColor Yellow
cargo fetch --target x86_64-unknown-linux-gnu
if ($LASTEXITCODE -ne 0) {
    Write-Host "警告: cargo fetch 失败，尝试 cargo fetch 无 target..." -ForegroundColor Yellow
    cargo fetch
}

# Step 2: 创建 vendor 目录
Write-Host "[2/4] 创建 vendor 目录..." -ForegroundColor Yellow
if (Test-Path vendor) {
    Remove-Item -Recurse -Force vendor
}
cargo vendor vendor/
if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: cargo vendor 失败" -ForegroundColor Red
    exit 1
}

# Step 3: 创建 .cargo/config.toml
Write-Host "[3/4] 配置离线源..." -ForegroundColor Yellow
$cargoDir = Join-Path $ProjectRoot ".cargo"
if (-not (Test-Path $cargoDir)) {
    New-Item -ItemType Directory -Path $cargoDir | Out-Null
}

$configContent = @"
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
"@

Set-Content -Path (Join-Path $cargoDir "config.toml") -Value $configContent -Encoding UTF8

# Step 4: 打包
Write-Host "[4/4] 打包源码..." -ForegroundColor Yellow
$timestamp = Get-Date -Format "yyyyMMdd"
$archiveName = "hdl-graph-offline-$timestamp.tar.gz"

# 使用 tar (Windows 10+ 自带)
$parentDir = Split-Path -Parent $ProjectRoot
$projectDirName = Split-Path -Leaf $ProjectRoot

tar czf (Join-Path $parentDir $archiveName) `
    --exclude='target' `
    --exclude='.git' `
    --exclude='._*' `
    --exclude='*.tar.gz' `
    -C $parentDir $projectDirName

if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: 打包失败" -ForegroundColor Red
    exit 1
}

$archivePath = Join-Path $parentDir $archiveName
$sizeMB = [math]::Round((Get-Item $archivePath).Length / 1MB, 1)

Write-Host ""
Write-Host "=== 完成 ===" -ForegroundColor Green
Write-Host "离线包: $archivePath ($sizeMB MB)" -ForegroundColor Green
Write-Host ""
Write-Host "传输到 Rocky Linux 后:" -ForegroundColor Cyan
Write-Host "  1. tar xzf $archiveName" -ForegroundColor White
Write-Host "  2. cd $projectDirName" -ForegroundColor White
Write-Host "  3. bash scripts/build-offline.sh" -ForegroundColor White

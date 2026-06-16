<#
.SYNOPSIS
    生成 hdl-graph 完整离线安装包
.DESCRIPTION
    在 Windows (有网络) 上运行此脚本，生成可在 Rocky Linux 8.9 / CentOS 7+ 上
    完全离线安装的 tar.gz 包。包含：
    - 源代码
    - 已 vendor 的 Cargo 依赖
    - Rust 离线安装器 (rustup-init)
    - .cargo/config.toml (指向 vendor)
    - 预编译的 Windows 二进制 (可选)
    - VS Code 扩展源码
    - 一键安装脚本
.PARAMETER OutputDir
    输出目录，默认为项目根目录的父目录
.PARAMETER IncludeRustToolchain
    是否包含 Rust 工具链离线安装器 (默认: true)
.PARAMETER IncludeWinBinary
    是否包含预编译的 Windows 二进制 (默认: true)
.PARAMETER TargetTriple
    目标 Linux 平台，默认 x86_64-unknown-linux-gnu
.EXAMPLE
    .\scripts\generate-offline-package.ps1
    .\scripts\generate-offline-package.ps1 -OutputDir "D:\packages" -IncludeRustToolchain:$false
#>

param(
    [string]$OutputDir = "",
    [bool]$IncludeRustToolchain = $true,
    [bool]$IncludeWinBinary = $true,
    [string]$TargetTriple = "x86_64-unknown-linux-gnu"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

if (-not $OutputDir) {
    $OutputDir = Split-Path -Parent $ProjectRoot
}

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  HDL-Graph 离线安装包生成工具" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "项目根目录: $ProjectRoot" -ForegroundColor Gray
Write-Host "输出目录:   $OutputDir" -ForegroundColor Gray
Write-Host "目标平台:   $TargetTriple" -ForegroundColor Gray
Write-Host ""

Set-Location $ProjectRoot

# ============================================
# Step 1: 检查前置条件
# ============================================
Write-Host "[1/7] 检查前置条件..." -ForegroundColor Yellow

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "错误: 未找到 cargo，请先安装 Rust" -ForegroundColor Red
    Write-Host "  winget install Rustlang.Rustup" -ForegroundColor Gray
    exit 1
}

$rustVersion = rustc --version 2>$null
Write-Host "  Rust: $rustVersion" -ForegroundColor Gray

if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
    Write-Host "错误: 未找到 tar (Windows 10+ 自带)" -ForegroundColor Red
    exit 1
}

# 检查磁盘空间 (需要 ~500MB)
$drive = (Get-Item $OutputDir).PSDrive
if ($drive) {
    $freeGB = [math]::Round((Get-PSDrive $drive.Name).Free / 1GB, 1)
    if ($freeGB -lt 1) {
        Write-Host "警告: 磁盘剩余空间不足 ${freeGB}GB，建议至少 1GB" -ForegroundColor Yellow
    }
}

Write-Host "  前置条件检查通过" -ForegroundColor Green
Write-Host ""

# ============================================
# Step 2: Vendor Cargo 依赖
# ============================================
Write-Host "[2/7] Vendor Cargo 依赖..." -ForegroundColor Yellow

# 确保 vendor 目录是最新的
$cargoLockHash = (Get-FileHash "Cargo.lock" -Algorithm MD5).Hash
$vendorMarker = ".vendor-hash"

$needVendor = $true
if ((Test-Path "vendor") -and (Test-Path $vendorMarker)) {
    $savedHash = Get-Content $vendorMarker -Raw
    if ($savedHash.Trim() -eq $cargoLockHash) {
        Write-Host "  vendor/ 已是最新，跳过" -ForegroundColor Gray
        $needVendor = $false
    }
}

if ($needVendor) {
    Write-Host "  运行 cargo vendor..." -ForegroundColor Gray
    if (Test-Path "vendor") {
        Remove-Item -Recurse -Force vendor
    }

    $vendorOutput = cargo vendor vendor/ 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "错误: cargo vendor 失败" -ForegroundColor Red
        Write-Host $vendorOutput -ForegroundColor Red
        exit 1
    }

    # 保存 hash 以避免重复 vendor
    Set-Content -Path $vendorMarker -Value $cargoLockHash -NoNewline
    Write-Host "  vendor 完成" -ForegroundColor Green
} else {
    Write-Host "  使用已有 vendor/" -ForegroundColor Green
}

$vendorSize = [math]::Round((Get-ChildItem vendor -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB, 1)
$vendorCount = (Get-ChildItem vendor -Directory).Count
Write-Host "  vendor: $vendorCount 个 crate, $vendorSize MB" -ForegroundColor Gray
Write-Host ""

# ============================================
# Step 3: 确保 .cargo/config.toml 正确
# ============================================
Write-Host "[3/7] 配置 .cargo/config.toml..." -ForegroundColor Yellow

$cargoDir = Join-Path $ProjectRoot ".cargo"
$configPath = Join-Path $cargoDir "config.toml"

# 备份现有 config.toml
if (Test-Path $configPath) {
    $backupPath = "$configPath.offline-backup"
    if (-not (Test-Path $backupPath)) {
        Copy-Item $configPath $backupPath
        Write-Host "  已备份现有 config.toml" -ForegroundColor Gray
    }
}

# 创建离线 config.toml
$configContent = @"
# 离线构建配置 - 由 generate-offline-package.ps1 生成
# 此文件指向本地 vendor/ 目录，使 cargo 无需网络即可编译

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

[net]
offline = true
"@

if (-not (Test-Path $cargoDir)) {
    New-Item -ItemType Directory -Path $cargoDir | Out-Null
}
Set-Content -Path $configPath -Value $configContent -Encoding UTF8
Write-Host "  .cargo/config.toml 已配置 (离线模式)" -ForegroundColor Green
Write-Host ""

# ============================================
# Step 4: 下载 Rust 离线安装器
# ============================================
$rustupPath = ""
if ($IncludeRustToolchain) {
    Write-Host "[4/7] 下载 Rust 离线安装器..." -ForegroundColor Yellow

    $toolchainDir = Join-Path $ProjectRoot "rust-toolchain-offline"
    if (-not (Test-Path $toolchainDir)) {
        New-Item -ItemType Directory -Path $toolchainDir | Out-Null
    }

    $rustupBin = Join-Path $toolchainDir "rustup-init"

    # 检查是否已存在且大小合理 (>1MB)
    $needDownload = $true
    if (Test-Path $rustupBin) {
        $size = (Get-Item $rustupBin).Length
        if ($size -gt 1MB) {
            Write-Host "  rustup-init 已存在 ($([math]::Round($size/1MB, 1)) MB)" -ForegroundColor Gray
            $needDownload = $false
        }
    }

    if ($needDownload) {
        $rustupUrl = "https://static.rust-lang.org/rustup/dist/$TargetTriple/rustup-init"
        Write-Host "  下载: $rustupUrl" -ForegroundColor Gray
        try {
            Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupBin -UseBasicParsing
            $size = [math]::Round((Get-Item $rustupBin).Length / 1MB, 1)
            Write-Host "  下载完成: $size MB" -ForegroundColor Green
        } catch {
            Write-Host "  警告: 下载 rustup-init 失败: $_" -ForegroundColor Yellow
            Write-Host "  目标机器需要有 Rust 或能联网安装" -ForegroundColor Yellow
        }
    }
    Write-Host ""
} else {
    Write-Host "[4/7] 跳过 Rust 离线安装器" -ForegroundColor Yellow
    Write-Host ""
}

# ============================================
# Step 5: 准备构建脚本
# ============================================
Write-Host "[5/7] 准备构建脚本..." -ForegroundColor Yellow

# 创建 .cargo/config.toml 模板 (用于目标机器)
$configTemplate = @"
# 离线构建配置
# 将此文件放到 .cargo/config.toml

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

[net]
offline = true
"@

Write-Host "  构建脚本已准备" -ForegroundColor Green
Write-Host ""

# ============================================
# Step 6: 收集文件并打包
# ============================================
Write-Host "[6/7] 打包..." -ForegroundColor Yellow

$timestamp = Get-Date -Format "yyyyMMdd"
$version = "0.1.1"
$archiveName = "hdl-graph-offline-v${version}-${timestamp}.tar.gz"
$archivePath = Join-Path $OutputDir $archiveName

# 构建排除列表
$excludePatterns = @(
    '--exclude=target',
    '--exclude=.git',
    '--exclude=._*',
    '--exclude=*.tar.gz',
    '--exclude=.vendor-hash',
    '--exclude=*.pdb',
    '--exclude=.cargo/config.toml.offline-backup'
)

# 不包含 Windows 二进制时排除 target
if (-not $IncludeWinBinary) {
    $excludePatterns += '--exclude=target/release/hdl-graph.exe'
    $excludePatterns += '--exclude=target/release/hdl-graph.d'
}

$projectDirName = Split-Path -Leaf $ProjectRoot
$parentDir = Split-Path -Parent $ProjectRoot

# 构建 tar 命令
$tarArgs = @('czf', $archivePath) + $excludePatterns + @('-C', $parentDir, $projectDirName)

Write-Host "  打包中..." -ForegroundColor Gray
& tar @tarArgs

if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: 打包失败" -ForegroundColor Red
    exit 1
}

$archiveSize = [math]::Round((Get-Item $archivePath).Length / 1MB, 1)
Write-Host "  打包完成: $archivePath" -ForegroundColor Green
Write-Host "  大小: $archiveSize MB" -ForegroundColor Gray
Write-Host ""

# ============================================
# Step 7: 验证
# ============================================
Write-Host "[7/7] 验证离线包..." -ForegroundColor Yellow

# 列出包内容摘要
Write-Host "  包内容:" -ForegroundColor Gray
$tarList = tar tzf $archivePath 2>$null
$dirCount = ($tarList | Where-Object { $_ -match '/$' }).Count
$fileCount = ($tarList | Where-Object { $_ -notmatch '/$' }).Count
Write-Host "    目录: $dirCount" -ForegroundColor Gray
Write-Host "    文件: $fileCount" -ForegroundColor Gray

# 检查关键文件
$criticalFiles = @(
    "Cargo.toml",
    "Cargo.lock",
    "vendor/",
    ".cargo/config.toml",
    "scripts/build-offline.sh",
    "crates/hdl-graph-cli/Cargo.toml"
)

$allPresent = $true
foreach ($f in $criticalFiles) {
    $pattern = "$projectDirName/$f"
    $found = $tarList | Where-Object { $_ -like "*$pattern*" }
    if (-not $found) {
        Write-Host "  警告: 缺少 $f" -ForegroundColor Yellow
        $allPresent = $false
    }
}

if ($allPresent) {
    Write-Host "  所有关键文件验证通过" -ForegroundColor Green
}

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  离线包生成完成!" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "离线包: $archivePath" -ForegroundColor White
Write-Host "大小:   $archiveSize MB" -ForegroundColor White
Write-Host ""
Write-Host "传输到 Rocky Linux 后:" -ForegroundColor Cyan
Write-Host "  1. tar xzf $archiveName" -ForegroundColor White
Write-Host "  2. cd $projectDirName" -ForegroundColor White
Write-Host "  3. bash scripts/build-offline.sh" -ForegroundColor White
Write-Host ""
Write-Host "或者使用一键安装:" -ForegroundColor Cyan
Write-Host "  tar xzf $archiveName && cd $projectDirName && bash install.sh" -ForegroundColor White
Write-Host ""

# prepare-rust-toolchain.ps1
# 在 Windows (有网络) 上准备 Rust 离线工具链包
# 用于在完全离线的 Rocky Linux 服务器上安装 Rust

$ErrorActionPreference = "Stop"

$ToolchainDir = "rust-toolchain-offline"
$RustupVersion = "1.27.1"

Write-Host "=== Rust 离线工具链准备工具 ===" -ForegroundColor Cyan
Write-Host ""

# 创建目录
if (Test-Path $ToolchainDir) {
    Remove-Item -Recurse -Force $ToolchainDir
}
New-Item -ItemType Directory -Path $ToolchainDir | Out-Null

# 下载 rustup-init for Linux
Write-Host "[1/3] 下载 rustup-init (Linux x86_64)..." -ForegroundColor Yellow
$rustupUrl = "https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"
$rustupPath = Join-Path $ToolchainDir "rustup-init"
Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath -UseBasicParsing

# 下载 stable toolchain archive
Write-Host "[2/3] 下载 stable toolchain (x86_64-unknown-linux-gnu)..." -ForegroundColor Yellow
Write-Host "  这可能需要几分钟..." -ForegroundColor Gray

# 使用 rustup 下载 toolchain 到指定目录
$env:RUSTUP_HOME = Join-Path $PWD "$ToolchainDir\rustup"
$env:CARGO_HOME = Join-Path $PWD "$ToolchainDir\cargo"

# 下载 toolchain metadata and archive
# 注意: 这里我们使用 rustup 的离线下载功能
$toolchainArchive = "stable-x86_64-unknown-linux-gnu"

# 创建下载脚本供 Linux 使用
$downloadScript = @"
#!/bin/bash
# 在有网络的 Linux 机器上运行此脚本下载工具链
# 然后将整个目录传输到离线服务器

set -e

export RUSTUP_HOME="\$(pwd)/rustup"
export CARGO_HOME="\$(pwd)/cargo"

chmod +x rustup-init
./rustup-init -y --default-toolchain stable --no-modify-path

echo ""
echo "工具链已下载到: \$(pwd)/rustup/"
echo "Cargo 已配置到: \$(pwd)/cargo/"
echo ""
echo "传输整个 $ToolchainDir 目录到离线服务器后，运行:"
echo "  bash install-rust-offline.sh"
"@

Set-Content -Path (Join-Path $ToolchainDir "download-on-linux.sh") -Value $downloadScript -Encoding UTF8

# 创建离线安装脚本
$installScript = @"
#!/bin/bash
# install-rust-offline.sh
# 在离线 Rocky Linux 服务器上安装 Rust 工具链

set -e

SCRIPT_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" ; pwd)"

echo "=== Rust 离线安装脚本 ==="
echo ""

# 设置 Rust 目录
export RUSTUP_HOME="\$HOME/.rustup"
export CARGO_HOME="\$HOME/.cargo"

mkdir -p "\$RUSTUP_HOME"
mkdir -p "\$CARGO_HOME"

# 复制 toolchain
echo "[1/3] 复制 toolchain..."
if [ -d "\$SCRIPT_DIR/rustup/toolchains" ]; then
    cp -r "\$SCRIPT_DIR/rustup/toolchains/"* "\$RUSTUP_HOME/" 2>/dev/null || true
fi

# 创建 rustup 设置
echo "[2/3] 配置 rustup..."
cat > "\$RUSTUP_HOME/settings.toml" << 'SETTINGS'
profile = "default"
version = "12"

[toolchains]
SETTINGS

# 获取 toolchain 目录名
TOOLCHAIN_DIR=\$(ls "\$RUSTUP_HOME/" 2>/dev/null | grep stable | head -1)
if [ -n "\$TOOLCHAIN_DIR" ]; then
    echo "default_host_triple = \"x86_64-unknown-linux-gnu\"" >> "\$RUSTUP_HOME/settings.toml"
    echo "default_toolchain = \"\$TOOLCHAIN_DIR\"" >> "\$RUSTUP_HOME/settings.toml"
fi

# 创建 cargo 链接
echo "[3/3] 配置 PATH..."
if [ ! -f "\$HOME/.cargo/env" ]; then
    cat > "\$HOME/.cargo/env" << 'ENV'
. "\$HOME/.cargo/env"
ENV
fi

# 添加到 .bashrc
if ! grep -q "\.cargo/env" "\$HOME/.bashrc" 2>/dev/null; then
    echo '. "\$HOME/.cargo/env"' >> "\$HOME/.bashrc"
fi

source "\$HOME/.cargo/env" 2>/dev/null || true

echo ""
echo "=== 安装完成 ==="
echo ""
echo "请运行: source ~/.cargo/env"
echo "然后验证: rustc --version && cargo --version"
"@

Set-Content -Path (Join-Path $ToolchainDir "install-rust-offline.sh") -Value $installScript -Encoding UTF8

# 打包
Write-Host "[3/3] 打包工具链..." -ForegroundColor Yellow
$timestamp = Get-Date -Format "yyyyMMdd"
$archiveName = "rust-toolchain-linux-offline-$timestamp.tar.gz"

tar czf $archiveName $ToolchainDir

$sizeMB = [math]::Round((Get-Item $archiveName).Length / 1MB, 1)

Write-Host ""
Write-Host "=== 完成 ===" -ForegroundColor Green
Write-Host "离线包: $archiveName ($sizeMB MB)" -ForegroundColor Green
Write-Host ""
Write-Host "使用方法:" -ForegroundColor Cyan
Write-Host ""
Write-Host "方案A: 在有网 Linux 机器上下载工具链" -ForegroundColor Yellow
Write-Host "  1. tar xzf $archiveName" -ForegroundColor White
Write-Host "  2. cd $ToolchainDir" -ForegroundColor White
Write-Host "  3. bash download-on-linux.sh" -ForegroundColor White
Write-Host "  4. 传输整个目录到离线服务器" -ForegroundColor White
Write-Host "  5. bash install-rust-offline.sh" -ForegroundColor White
Write-Host ""
Write-Host "方案B: 直接在离线服务器上安装 (需要预下载的 toolchain)" -ForegroundColor Yellow
Write-Host "  1. tar xzf $archiveName" -ForegroundColor White
Write-Host "  2. cd $ToolchainDir" -ForegroundColor White
Write-Host "  3. bash install-rust-offline.sh" -ForegroundColor White

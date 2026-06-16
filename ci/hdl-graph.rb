# Homebrew formula for hdl-graph
# brew install hdl-graph/tap/hdl-graph

class HdlGraph < Formula
  desc "HDL Code Graph �?Verilog/SystemVerilog/UVM code intelligence"
  homepage "https://github.com/lixiaoxin/hdl-codegraph"
  license "MIT"
  version "0.2.0"

  on_macos do
    on_arm do
      url "https://github.com/lixiaoxin/hdl-codegraph/releases/download/v0.1.0/hdl-graph-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000" # placeholder
    end
    on_intel do
      url "https://github.com/lixiaoxin/hdl-codegraph/releases/download/v0.1.0/hdl-graph-x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000" # placeholder
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/lixiaoxin/hdl-codegraph/releases/download/v0.1.0/hdl-graph-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000" # placeholder
    end
    on_intel do
      url "https://github.com/lixiaoxin/hdl-codegraph/releases/download/v0.1.0/hdl-graph-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000" # placeholder
    end
  end

  def install
    bin.install "hdl-graph"
  end

  test do
    system "#{bin}/hdl-graph", "version"
  end
end

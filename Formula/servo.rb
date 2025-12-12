class Servo < Formula
  desc "Servo browser engine with WASM and TypeScript support"
  homepage "https://github.com/pannous/servo"
  url "https://github.com/pannous/servo.git", branch: "main"
  version "1.0.0"
  head "https://github.com/pannous/servo.git", branch: "main"

  depends_on "rust" => :build
  depends_on "cmake" => :build
  depends_on "pkg-config" => :build
  depends_on "python@3.11" => :build

  def install
    system "./mach", "build", "--release"
    bin.install "target/release/servo"
  end

  test do
    system "#{bin}/servo", "--version"
  end
end

class Schemaui < Formula
  desc "Render JSON Schemas as TUIs and embedded web editors"
  homepage "https://github.com/YuniqueUnic/schemaui"
  url "https://github.com/YuniqueUnic/schemaui/archive/refs/tags/schemaui-cli-v0.7.3.tar.gz"
  sha256 "888d2801f2cf769e56b1b0d1a598084be7036e3c165f8e9e00b8e11a2091a6d6"
  license "MIT OR Apache-2.0"
  head "https://github.com/YuniqueUnic/schemaui.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "schemaui-cli"), "--features", "full"
  end

  test do
    assert_equal "schemaui #{version}\n", shell_output("#{bin}/schemaui --version")
  end
end

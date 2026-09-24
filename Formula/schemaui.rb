class Schemaui < Formula
  desc "Render JSON Schemas as TUIs and embedded web editors"
  homepage "https://github.com/YuniqueUnic/schemaui"
  url "https://github.com/YuniqueUnic/schemaui/archive/refs/tags/schemaui-cli-v0.9.2.tar.gz"
  sha256 "0c426f7455f3eac2b82c5818e0f0ed14834e99260187c02137d9eb2d098f932f"
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

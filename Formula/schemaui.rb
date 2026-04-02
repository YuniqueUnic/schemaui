class Schemaui < Formula
  desc "Render JSON Schemas as TUIs and embedded web editors"
  homepage "https://github.com/YuniqueUnic/schemaui"
  url "https://github.com/YuniqueUnic/schemaui/archive/refs/tags/schemaui-cli-v0.4.1.tar.gz"
  sha256 "4be9a5912fcb12c6c32fcea0aa04fc9d376e387cb3a42aa416c0cb652c1cf7c7"
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

class Symtool < Formula
  version '0.1.2'
  desc "Manipulate static symbols in ELF and Mach-O objects"
  homepage "https://github.com/calebzulawski/symtool"

  if OS.mac?
    url "https://github.com/calebzulawski/symtool/releases/download/#{version}/symtool-macos-x86_64.tar.gz"
    sha256 "3ae85c3a613d19f52f879821b43941e8daa2c01dfad98fd709c045bb5bf7c187"
  elsif OS.linux?
    url "https://github.com/calebzulawski/symtool/releases/download/#{version}/symtool-linux-x86_64.tar.gz"
    sha256 "f0ea98b2e99fb4694aaedf63d680d26923b0a625862e84348115a69c34790e6f"
  end

  def install
    bin.install "symtool"
    man1.install "doc/symtool.1"
  end
end

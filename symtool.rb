class Symtool < Formula
  version '0.1.4'
  desc "Manipulate static symbols in ELF and Mach-O objects"
  homepage "https://github.com/calebzulawski/symtool"

  if OS.mac?
    url "https://github.com/calebzulawski/symtool/releases/download/#{version}/symtool-macos-x86_64.tar.gz"
    sha256 "c08ee3b50eff66e1dd536e7cdf28595f8270015ae206049a5733dca102dc9b57"
  elsif OS.linux?
    url "https://github.com/calebzulawski/symtool/releases/download/#{version}/symtool-linux-x86_64.tar.gz"
    sha256 "6e85befff3aefd186313d3ad5a6a47600fe473b6772b07ea0f34464c23343035"
  end

  def install
    bin.install "symtool"
    man1.install "doc/symtool.1"
  end
end

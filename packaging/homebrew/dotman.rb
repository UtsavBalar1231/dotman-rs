# Documentation: https://docs.brew.sh/Formula-Cookbook
#                https://rubydoc.brew.sh/Formula
class Dotman < Formula
  desc "High-performance dotfiles manager with git-like semantics"
  homepage "https://github.com/UtsavBalar1231/dotman"
  url "https://github.com/UtsavBalar1231/dotman/archive/refs/tags/v0.0.1.tar.gz"
  sha256 "SKIP"  # Updated by CI/release workflow
  license "MIT"
  head "https://github.com/UtsavBalar1231/dotman.git", branch: "main"

  depends_on "rust" => :build
  depends_on "help2man" => :build

  def install
    system "cargo", "install", "--locked", "--root", prefix, "--path", "."

    # Generate and install shell completions
    generate_completions_from_executable(bin/"dot", "completion")

    # Generate and install man page
    system "help2man", "--no-info",
           "--name=high-performance dotfiles manager",
           "--version-string=#{version}",
           bin/"dot",
           "--output=dot.1"
    man1.install "dot.1"
  end

  test do
    # Test that the binary runs and shows version
    assert_match version.to_s, shell_output("#{bin}/dot --version")

    # Test basic functionality
    system bin/"dot", "init"
    assert_predicate testpath/".dotman", :exist?

    system bin/"dot", "status"
    system bin/"dot", "--help"
  end
end

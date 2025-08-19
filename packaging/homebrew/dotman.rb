# Documentation: https://docs.brew.sh/Formula-Cookbook
#                https://rubydoc.brew.sh/Formula
class Dotman < Formula
  desc "Blazingly fast dotfiles manager with git-like semantics"
  homepage "https://github.com/UtsavBalar1231/dotman-rs"
  url "https://github.com/UtsavBalar1231/dotman-rs/archive/refs/tags/v0.0.1.tar.gz"
  sha256 "" # This will be updated when the release is created
  license "MIT"
  head "https://github.com/UtsavBalar1231/dotman-rs.git", branch: "main"

  # Minimum macOS version requirement
  depends_on macos: :high_sierra

  # Build dependencies
  depends_on "rust" => :build
  depends_on "help2man" => :build

  # Runtime dependencies (none required for static binary)

  def install
    # Set optimization flags for native CPU
    ENV["RUSTFLAGS"] = "-C target-cpu=native -C opt-level=3 -C lto=fat -C strip=symbols"
    
    # Build the project
    system "cargo", "install", "--locked", "--root", prefix, "--path", "."
    
    # Generate shell completions
    mkdir_p "completions"
    system bin/"dot", "completion", "bash", "> completions/dot.bash"
    system bin/"dot", "completion", "zsh", "> completions/_dot"  
    system bin/"dot", "completion", "fish", "> completions/dot.fish"
    
    # Install completions
    bash_completion.install "completions/dot.bash" => "dot"
    zsh_completion.install "completions/_dot"
    fish_completion.install "completions/dot.fish"
    
    # Generate and install man page
    system "help2man", "--no-info", "--name=blazingly fast dotfiles manager", 
           "--version-string=#{version}", "#{bin}/dot", "--output=dot.1"
    man1.install "dot.1"
  end

  test do
    # Test that the binary runs and shows version
    assert_match version.to_s, shell_output("#{bin}/dot --version")
    
    # Test basic functionality with a temporary directory
    system bin/"dot", "init"
    assert_predicate testpath/".dotman", :exist?
    
    # Test status command
    system bin/"dot", "status"
    
    # Test help command
    system bin/"dot", "--help"
  end
end

# Alternative formula for development version
class DotmanHead < Formula
  desc "Blazingly fast dotfiles manager with git-like semantics (HEAD)"
  homepage "https://github.com/UtsavBalar1231/dotman-rs"
  head "https://github.com/UtsavBalar1231/dotman-rs.git", branch: "main"
  license "MIT"

  depends_on macos: :high_sierra
  depends_on "rust" => :build
  depends_on "help2man" => :build

  def install
    ENV["RUSTFLAGS"] = "-C target-cpu=native -C opt-level=3 -C lto=fat -C strip=symbols"
    
    system "cargo", "install", "--locked", "--root", prefix, "--path", "."
    
    # Generate shell completions
    mkdir_p "completions"
    system bin/"dot", "completion", "bash", "> completions/dot.bash"
    system bin/"dot", "completion", "zsh", "> completions/_dot"
    system bin/"dot", "completion", "fish", "> completions/dot.fish"
    
    bash_completion.install "completions/dot.bash" => "dot"
    zsh_completion.install "completions/_dot"
    fish_completion.install "completions/dot.fish"
    
    # Generate and install man page
    system "help2man", "--no-info", "--name=blazingly fast dotfiles manager",
           "--version-string=HEAD", "#{bin}/dot", "--output=dot.1"
    man1.install "dot.1"
  end

  test do
    assert_match(/\d+\.\d+\.\d+/, shell_output("#{bin}/dot --version"))
    system bin/"dot", "init"
    assert_predicate testpath/".dotman", :exist?
    system bin/"dot", "status"
    system bin/"dot", "--help"
  end
end
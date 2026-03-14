class Siphon < Formula
  desc "Developer content generation tool — track activity, surface content ideas"
  homepage "https://github.com/kevinreber/siphon"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kevinreber/siphon/releases/download/v0.1.0/siphon-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/kevinreber/siphon/releases/download/v0.1.0/siphon-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    url "https://github.com/kevinreber/siphon/releases/download/v0.1.0/siphon-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "PLACEHOLDER"
  end

  depends_on "node"

  def install
    bin.install "bin/siphon-daemon"
    bin.install "bin/siphon-ctl"

    # Install CLI JS bundle and wrapper
    libexec.install "siphon-cli-dist"
    (bin/"siphon").write_env_script libexec/"siphon-cli-dist/cli.js", PATH: "#{Formula["node"].opt_bin}:$PATH"

    # Install UI files
    (share/"siphon-ui").install Dir["share/siphon-ui/*"]

    # Install shell hooks
    (share/"siphon/hooks").install Dir["share/hooks/*"]
  end

  def post_install
    (var/"siphon").mkpath
  end

  def caveats
    <<~EOS
      To start capturing activity, run:
        brew services start siphon

      Then open the dashboard at:
        http://localhost:9847

      To set up shell hooks for command tracking, add to your ~/.zshrc:
        source #{share}/siphon/hooks/siphon-hook.zsh
    EOS
  end

  service do
    run [opt_bin/"siphon-daemon"]
    keep_alive true
    log_path var/"log/siphon-daemon.log"
    error_log_path var/"log/siphon-daemon.log"
  end

  test do
    assert_match "siphon", shell_output("#{bin}/siphon-ctl --help")
  end
end

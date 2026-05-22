dotty.link("foo.txt", "bar.txt")

if dotty.hostname() == "foo" and dotty.os() == "linux" then
  dotty.link("baz.txt", "qux.txt")
end

dotty.run("Install Zap for zsh", "zsh <(curl -s https://raw.githubusercontent.com/zap-zsh/zap/master/install.zsh) --branch release-v1")

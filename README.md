# Dotty

Manage (symlink) your dotfiles with Dotty. Uses Lua for configuration.

A successor to Dotman with Lua as config instead of TOML.

## Example

```lua
local hostname = dotty.hostname()
local os = dotty.os()
local profile = dotty.profile()

-- Always symlink
dotty.link("hosts/common/config/git", "~/.config/git")

-- Conditional links
if os == "macos" then
  dotty.link("hosts/mac/zshrc", "~/.zshrc")
end

if hostname == "work-laptop" then
  dotty.link("hosts/work/vimrc", "~/.vimrc")
end

-- Only install Zap if it's not already on the system
if not dotty.test("zap") then
  dotty.run("Install Zap for zsh", "zsh <(curl -s https://raw.githubusercontent.com/zap-zsh/zap/master/install.zsh) --branch release-v1")
end

-- Run with a specific shell
dotty.run("Setup fish plugins", { command = "fisher install jorgebucaran/autopair.fish", shell = "fish" })

-- Profile-based config
if profile == "work" then
  dotty.link("hosts/work/ssh-config", "~/.ssh/config")
end
```

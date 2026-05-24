# Dotty

Manage (symlink) your dotfiles with Dotty. Uses Lua for configuration.

A successor to Dotman with Lua as config instead of TOML.

## Example

```bash
# Declare dir prefixes
common = "hosts/common"
mac    = "hosts/mac"
work   = "hosts/work"

link "$common/config/git" to "~/.config/git"

if os is "macos" {
  link "$mac/zshrc" to "~/.zshrc"
}

if os is not "linux" {
  link "$common/config/karabiner" to "~/.config/karabiner"
}

if hostname is "work-laptop" {
  link "$work/vimrc" to "~/.vimrc"
}

# Only install Zap if it's not already on the system
if not test "zap" {
  do "zsh <(curl -s https://raw.githubusercontent.com/zap-zsh/zap/master/install.zsh) --branch release-v1"
}

# Run with a specific shell
do "fish" "fisher install jorgebucaran/autopair.fish"

# Profile-based config
if profile is "work" {
  link "$work/ssh-config" to "~/.ssh/config"
} else {
  link "$common/ssh-config" to "~/.ssh/config"
}
```

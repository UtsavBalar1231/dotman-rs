#!/usr/bin/env bash

function cargo_check() {
if ! command -v paru >/dev/null 2>&1; then
	if ! command -v cargo >/dev/null 2>&1; then
		if [ -d ~/.cargo/bin ]; then
			export PATH="$HOME/.cargo/bin:$PATH"
		fi

		cargo install paru
	fi
fi
}

# Check if $DISPLAY is set
active_monitors=$(xrandr | grep -c " connected")
if [ -n "$active_monitors" ]; then
	# Configure polybar: {{{
	sudo pacman --noconfirm -S polybar
	# }}}
fi

# NVIM configuration: {{{
if [ ! -d /root/.config ]; then
	sudo mkdir -p /root/.config
fi

if [ ! -d /root/.config/nvim ]; then
	sudo cp -afr ~/.config/nvim/ /root/.config/nvim
fi

nvim --headless +PackerSync +qa
# }}}

#: {{{ AUR packages
arch_aur_packages="
bibata-cursor-theme-bin
docker-desktop
dunst-git
feh-git
flameshot-git
google-chrome-dev
gruvbox-material-gtk-theme-git
gruvbox-material-icon-theme-git
gruvbox-plus-icon-theme-git
i3-git
i3lock-git
i3status-git
kitty-git
kitty-shell-integration-git
kitty-terminfo-git
libinput-gestures-git
libxfce4util-devel
nbfc-linux
nerd-fonts-inter
pavucontrol-git
pcmanfm-git
polybar-git
rofi-bluetooth-git
rofi-git
rofi-greenclip
simplescreenrecorder-bin
slack-desktop
wlr-protocols-git
xf86-input-libinput-git
xfce4-dev-tools-devel
xfce4-power-manager-git
xinit-xsession
xsecurelock-git
"

# Install AUR packages
cargo_check

IFS=' ' read -r -a packages <<< "$arch_aur_packages"
for package in "${packages[@]}"; do
	if ! paru -Qi "$package" >/dev/null 2>&1; then
		paru -S --noconfirm "$package"
	else
		echo "$package is already installed"
	fi
done
# }}}


# Configure zsh: {{{
sudo chsh "$(whoami)" -s "$(which zsh)"
sudo chsh -s "$(which zsh)"

if [ ! -d /root/.config/zsh ]; then
	sudo cp -afr ~/.config/zsh/ /root/.config/zsh
fi

# shellcheck disable=SC1090
source ~/.zshrc
# }}}

exec $(which zsh)

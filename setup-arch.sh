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

declare -a arch_base_packages=(
	acpi alacritty base base-devel bash bash-completion bat btop clang cmake cpio curl eza fastfetch fd firefox-nightly-bin
	fzf gcc git git-delta google-chrome-beta kdeconnect kitty libinput-gestures ly make meson neovim nodejs npm nvidia
	okular p7zip-full pyenv python python-pip python-pipenv python-pipx ripgrep rustup tmux ttf-terminus-nerd vim yay-bin
	yazi zen-twilight-avx2-bin zoxide zsh mpc iw
)

declare -a arch_xorg_packages=(
	bluez bluez-utils brightnessctl feh flameshot i3-git lxappearance lxrandr polybar-git rofi-emoji rofi-greenclip
	simplescreenrecorder-bin xclip xorg xsel
)

declare -a arch_rice_packages=(
	aurutils bibata-cursor-theme-bin diff-so-fancy gdrive-git getnf gruvbox-gtk-theme-git gruvbox-icon-theme-git gruvbox-plus-icon-theme
	gruvbox-wallpaper hollywood lazygit picom python-pywal16 python-pywalfox qt5ct qt6ct
)

declare -a arch_misc_packages=(
	lineageos-devel pacman-contrib seer-gdb-git spice-vdagent steam sunshine-bin tartube telegram-desktop texlive
	udiskie uget upwork valgrind ventoy-bin visual-studio-code-insiders-bin whatsapp-for-linux
)

declare -a arch_wayland_packages=(
	cliphist cosmic-comp-git cosmic-comp-git-debug cosmic-ext-calculator-git cosmic-ext-forecast-git cosmic-ext-tweaks-git
	cosmic-greeter-git cosmic-session-git cosmic-store-git grimblast-git hyprcursor-git hyprgraphics-git hypridle-git
	hyprland-git hyprlock-git hyprpaper-git hyprpicker hyprshade-git hyprsunset-git nwg-displays rofi-lbonn-wayland-git
	swaybg swaync swayosd-git swww system76-acpi-dkms-git system76-io-dkms-git system76-power system76-scheduler-git
	waybar wayvnc wl-clipboard xdg-desktop-portal-hyprland-git
)

dry_run() {
	local packages=("$@")
	local to_install=()

	for package in "${packages[@]}"; do
		if ! pacman -Qi "$package" >/dev/null 2>&1; then
			to_install+=("$package")
		else
			echo "$package is already installed"
		fi
	done

	echo "Packages to install: ${to_install[*]}"
	echo "${to_install[@]}"
}

install_packages() {
	local packages=("$@")
	if [ "${#packages[@]}" -gt 0 ]; then
		paru -S --noconfirm "${packages[@]}"
	else
		echo "No packages to install."
	fi
}

cargo_check
paru -Syu --noconfirm

all_packages=(
	"${arch_base_packages[@]}"
	"${arch_xorg_packages[@]}"
	"${arch_rice_packages[@]}"
	"${arch_misc_packages[@]}"
	"${arch_wayland_packages[@]}"
)

to_install=($(dry_run "${all_packages[@]}"))
install_packages "${to_install[@]}"

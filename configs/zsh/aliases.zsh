#/usr/bin/env zsh

function get_xdg_session_type() {
	echo $XDG_SESSION_TYPE
}

function get_package_manager() {
	# Check /etc/os-release
	ID=$(grep '^ID=' /etc/os-release | cut -d'=' -f2)

	case $ID in
		'arch')
			if command -v paru >/dev/null 2>&1; then
				echo 'paru'
			elif command -v yay >/dev/null 2>&1; then
				echo 'yay'
			else
				echo 'pacman'
			fi
			;;
		'fedora')
			echo 'dnf'
			;;
		'debian' | 'ubuntu')
			echo 'apt'
			;;
		*)
			echo 'unknown'
			;;
	esac
}

# $1: package name in fedora, $2: package name in arch, $3: package name in apt
function install_package() {
	echo "installing $1 using $package_manager"
	if [ $1 -eq "" ]; then
		echo "Ignoring empty package name"
		return
	fi
	case $package_manager in
		'dnf')
			sudo dnf install $1
			;;
		'apt')
			sudo apt install $3
			;;
		'pacman')
			sudo pacman -S --noconfirm $2
			;;
		*)
			echo "Please install $2" >&2
			;;
	esac
}

package_manager=$(get_package_manager)

# alias for ls
if command -v eza >/dev/null 2>&1; then
	eza_params=('--icons' '--classify' '--group-directories-first' '--time-style=long-iso' '--group' '--color-scale' '--sort=modified')

	alias ls='eza $eza_params'
	alias l='eza --git-ignore $eza_params'
	alias ll='eza --all --header --long $eza_params'
	alias llm='eza --all --header --long --sort=modified $eza_params'
	alias la='eza -lbhHigUmuSa'
	alias lx='eza -lbhHigUmuSa@'
	alias lt='eza --tree $eza_params'
	alias tree='eza --tree $eza_params'
else
	install_package 'eza' 'eza' 'eza'
fi

# alias for bat
if command -v bat >/dev/null 2>&1; then
	alias b='bat'
elif command -v batcat >/dev/null 2>&1; then
	alias bat='batcat'
	alias b='bat'
else
	install_package 'bat' 'bat' 'bat'
fi

# alias for rg
if command -v rg >/dev/null 2>&1; then
	# alias rg='rg --smart-case'
	alias rgf='rg --files'
	alias rgd='rg --files-with-matches'
else
	install_package 'ripgrep' 'ripgrep' 'ripgrep'
fi

if command -v fd >/dev/null 2>&1; then
	alias fd='fd --color=always'
elif command -v fdfind >/dev/null; then
	alias fd='fdfind --color=always'
else
	install_package 'fd' 'fd-find' 'fd-find'
fi

if command -v nvim >/dev/null 2>&1; then
	alias v='nvim'
	alias n='nvim'
else
	if [ $(get_package_manager) -eq 'apt' ]; then
		sudo add-apt-repository ppa:neovim-ppa/unstable -y
		sudo apt update -y
	fi

	install_package 'neovim' 'neovim' 'neovim'
fi

if !command -v git>/dev/null 2>&1; then
	install_package 'git' 'git' 'git'
fi

# git aliases
alias gb='git branch'
alias gc='git clone'
alias gca='git commit --amend'
alias gch='git checkout'
alias gcl='git clean'
alias gcm='git commit'
alias gcp='git cherry-pick'
alias gd='git diff'
alias gdc='git diff --cached'
alias gf='git fetch'
alias gl1='git log --oneline'
alias gl='git log'
alias gp='git pull'
alias gps='git push'
alias gr='git revert'
alias grb='git rebase'
alias grf='git reflog'
alias grm='git remote'
alias grst='git reset'
alias grsth='git reset --hard'
alias gs='git status'

# alias for sudo
alias sudo='sudo '

# alias for ssh
alias ssh="TERM=xterm-256color ssh"

if command -v gdrive >/dev/null 2>&1; then
	function gdr() {
		case $1 in
			'upload'|'-u')
				shift
				echo "Uploading... $@"
				output=$(gdrive files upload "$@" 2>&1)
				id=$(echo "$output" | grep 'Id:' | awk '{print $2}')
				url=$(echo "$output" | grep 'ViewUrl:' | awk '{print $2}')
				if [[ ! -z $id ]]; then
					echo "ID: $id"
					gdrive permissions share "$id"
				fi
					echo "URL: $url"
				;;
			'list'|'-l')
				shift
				gdrive list $@
				;;
			*)
				gdrive $@
				;;
		esac
	}
fi

# alias for xdg
alias x="xdg-open"

# alias for kitty graph visualization
if command -v dot >/dev/null 2>&1; then
	alias idot="dot -T png | kitty +kitten icat"
fi

# alias for clipboard
if [ $(get_xdg_session_type) = 'wayland' ]; then
	if command -v wl-copy >/dev/null 2>&1; then
		alias clip='wl-copy'
	else
		install_package 'wl-clipboard' 'wl-clipboard' 'wl-clipboard'
	fi
elif [ $(get_xdg_session_type) = 'x11' ]; then
	if command -v xclip >/dev/null 2>&1; then
		alias clip='xclip -selection clipboard'
	else
		install_package 'xclip' 'xclip' 'xclip'
		install_package 'xsel' 'xsel' 'xsel'
	fi
fi

# vim like shutdown
alias :wq='sudo shutdown -h now'

# alias for terminal file manager
if command -v yazi >/dev/null 2>&1; then
	alias y='yazi'
fi

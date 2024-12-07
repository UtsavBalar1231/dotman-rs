#/usr/bin/env zsh

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
else
	if command -v fdfind >/dev/null; then
		alias fd='fdfind --color=always'
	fi
fi

# alias for vim
alias n='nvim'
alias v='vim'

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
alias idot="dot -T png | kitty +kitten icat"

# alias for xclip clipboard
alias clip='xclip -selection clipboard'

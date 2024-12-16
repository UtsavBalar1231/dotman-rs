#zmodload zsh/zprof

# Theme
if ! command -v starship >/dev/null; then
	curl -sS https://starship.rs/install.sh | sh
fi
export STARSHIP_CONFIG=${HOME}/.config/starship.toml
eval "$(starship init zsh)"

# Enable history
HISTSIZE=100000
SAVEHIST=$HISTSIZE
HISTFILE=${HOME}/.zsh_history
HISTDUP=erase

setopt appendhistory
setopt sharehistory
setopt hist_ignore_space
setopt hist_ignore_all_dups
setopt hist_save_no_dups
setopt hist_ignore_dups
setopt hist_find_no_dups

# Enable auto completion
#setopt auto_menu
#setopt auto_list
#setopt auto_param_keys
#setopt auto_param_slash
#setopt auto_remove_slash
#setopt autocd
#setopt auto_pushd

setopt correct

autoload -Uz compinit && compinit
autoload -Uz +X bashcompinit && bashcompinit
zstyle :compinstall filename '${HOME}/.zshrc'
zstyle ':completion:*' matcher-list 'm:{a-z}={A-Za-z}'
# disable sort when completing `git checkout`
zstyle ':completion:*:git-checkout:*' sort false
# set descriptions format to enable group support
# NOTE: don't use escape sequences here, fzf-tab will ignore them
zstyle ':completion:*:descriptions' format '[%d]'
# set list-colors to enable filename colorizing
zstyle ':completion:*' list-colors ${(s.:.)LS_COLORS}
# force zsh not to show completion menu, which allows fzf-tab to capture the unambiguous prefix
zstyle ':completion:*' menu no
# preview directory's content with eza when completing cd
zstyle ':fzf-tab:complete:cd:*' fzf-preview 'eza -1 --color=always $realpath'
# switch group using `<` and `>`
zstyle ':fzf-tab:*' switch-group '<' '>'

# Enable aliases
setopt aliases
source ${HOME}/.config/zsh/aliases.zsh

# key bindings
bindkey -v

bindkey '^[[1;5C' forward-word
bindkey '^[[1;5D' backward-word
bindkey '^[[F' end-of-line
bindkey '^[[H' beginning-of-line
bindkey '^[[3~' delete-word
bindkey '^[[2~' kill-line
bindkey '^[[3~' delete-char

bindkey -M viins "^E" end-of-line
bindkey -M viins "^A" beginning-of-line
bindkey -M viins "^P" history-search-backward
bindkey -M viins "^N" history-search-forward

if command -v zoxide >/dev/null; then
	eval "$(zoxide init zsh)"
fi


# List of plugins with optional third parameter for specific source file
declare -g plugins=(
    "z-shell/F-Sy-H:main"
    "zsh-users/zsh-autosuggestions:develop"
    "zsh-users/zsh-completions:master"
    "zsh-users/zsh-history-substring-search:master"
    "zsh-users/zsh-syntax-highlighting:master"
    "Aloxaf/fzf-tab:master:fzf-tab.plugin"
)

# Loop through each plugin entry
for plugin in "${plugins[@]}"; do
    plugin_name="${plugin%%:*}"
    rest="${plugin#*:}"
    plugin_branch="${rest%%:*}"
    plugin_file="${rest#*:}"

    # echo "Loading $plugin_name, branch: $plugin_branch, file: $plugin_file"
    
    # Determine the plugin directory
    plugin_dir="${HOME}/.config/zsh/plugins/${plugin_name}"

    # Clone the plugin if it doesn't exist
    if [ ! -d "${plugin_dir}" ]; then
        git clone --depth=1 -b "$plugin_branch" "https://github.com/${plugin_name}" "${plugin_dir}"
    fi

    # Determine the file to source
    if [ "$plugin_file" != "$plugin_branch" ]; then
        # If a specific source file is provided, use it
        zsh_file="${plugin_dir}/${plugin_file}.zsh"
    else
        # Otherwise, find the first .zsh file in the plugin directory
        zsh_file=$(find "${plugin_dir}" -maxdepth 1 -type f -name "*.zsh" | head -n 1)
    fi

    # Source the file if it exists
    if [ -f "${zsh_file}" ]; then
        source "${zsh_file}"
    else
        echo "Warning: No .zsh file found for plugin ${plugin_name}"
    fi
done

# zsh-autosuggestions

# Disable automatic widget re-binding on each precmd. This can be set when
# zsh-users/zsh-autosuggestions is the last module in your ~/.zimrc.
ZSH_AUTOSUGGEST_MANUAL_REBIND=1

# Customize the style that the suggestions are shown with.
# See https://github.com/zsh-users/zsh-autosuggestions/blob/master/README.md#suggestion-highlight-style
ZSH_AUTOSUGGEST_HIGHLIGHT_STYLE='bold'


# zsh-syntax-highlighting

# Set what highlighters will be used.
# See https://github.com/zsh-users/zsh-syntax-highlighting/blob/master/docs/highlighters.md
ZSH_HIGHLIGHT_HIGHLIGHTERS=(main brackets)

# Customize the main highlighter styles.
# See https://github.com/zsh-users/zsh-syntax-highlighting/blob/master/docs/highlighters/main.md#how-to-tweak-it
typeset -A ZSH_HIGHLIGHT_STYLES
ZSH_HIGHLIGHT_STYLES[comment]='fg=242'

# Function to display the progress bar
show_progress_bar() {
	local progress=$1
	local bar_length=30
	local filled_length=$((progress * bar_length / 100))
	local empty_length=$((bar_length - filled_length))

	local bar
	bar=$(printf "%0.s█" $(seq 1 "$filled_length"))
	bar+=$(printf "%0.s▒" $(seq 1 "$empty_length"))

	printf "\rUpdating '%s': [%s] %d%%" "$current_plugin" "$bar" "$progress"
}

# Function to clone a repository with a progress bar
clone_repo_with_progress() {
	local repo_url=$1
	local dest_dir=$2
	local branch=$3

	git clone --progress "$repo_url" "$dest_dir" -b "$branch" 2>&1 | while IFS= read -r line; do
		if [[ $line =~ ([0-9]+)% ]]; then
			local progress=${BASH_REMATCH[1]}
			show_progress_bar "$progress"
		fi
	done

	show_progress_bar 100
}

# Cargo environment
if [ -f ${HOME}/.cargo/env ]; then
	source ${HOME}/.cargo/env
else
	if [ -d ${HOME}/.cargo/bin ]; then
		export PATH="${HOME}/.cargo/bin:${PATH}"
	fi
fi

# Rust
if ! command -v rustc >/dev/null; then
	curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
fi

# Python binaries
if [ -d ${HOME}/.local/bin ]; then
	export PATH="${HOME}/.local/bin":${PATH}
fi

# Mason binaries
if [ -d ${HOME}/.local/share/nvim/mason/bin ]; then
	export PATH="${HOME}/.local/share/nvim/mason/bin":${PATH}
fi

# Gitlint
if [ -f ${HOME}/.gitlint ]; then
	GITLINT_CONFIG=${HOME}/.gitlint
	export GITLINT_CONFIG
fi

# custom Golang
if [ -d "/usr/local/go" ]; then
	export GOARCH=amd64
	export GOOS=linux
	export GOROOT=/usr/local/go
	export PATH=$GOROOT/bin:$PATH
fi

# Edit command line
autoload -Uz edit-command-line
zle -N edit-command-line
bindkey '^X^E' edit-command-line
fpath+=${ZDOTDIR:-${HOME}}/.zsh_functions

# Set default editor
if command -v nvim >/dev/null 2>&1; then
	export EDITOR=nvim
fi

# Broot
if command -v broot >/dev/null 2>&1; then
	source ${HOME}/.config/broot/launcher/bash/br
fi

# ZSH plugins
function update_zsh_plugins() {
	for plugin in "${plugins[@]}"; do
		plugin_name="${plugin%%:*}"
    	rest="${plugin#*:}"
    	plugin_branch="${rest%%:*}"
    	plugin_file="${rest#*:}"

		echo "Updating $plugin_name" >&2
		rm -rf "$HOME/.config/zsh/plugins/$plugin_name"

		clone_repo_with_progress "https://github.com/$plugin_name" "$HOME/.config/zsh/plugins/$plugin_name" "$plugin_branch"
	done
}

# FZF
if [ ! -f ${HOME}/.fzf.zsh ]; then
	clone_repo_with_progress "https://github.com/junegunn/fzf" ${HOME}/.fzf
	${HOME}/.fzf/install
fi
eval "$(fzf --zsh)"

# Pyenv
if [ -d ${HOME}/.pyenv ]; then
	export PYENV_ROOT="$HOME/.pyenv"
	[[ -d $PYENV_ROOT/bin ]] && export PATH="$PYENV_ROOT/bin:$PATH"
	eval "$(pyenv init -)"
	eval "$(pyenv virtualenv-init -)"
fi

if [ -f ${HOME}/dev/toolchains/.env ]; then
	source ${HOME}/dev/toolchains/.env
fi

# bun completions
[ -s "/home/utsav/.bun/_bun" ] && source "/home/utsav/.bun/_bun"

# bun
export BUN_INSTALL="$HOME/.bun"
export PATH="$BUN_INSTALL/bin:$PATH"

# rofi
export PATH=$HOME/.config/rofi/scripts:$PATH

# Local binaries
if [ -d $HOME/.local/bin ]; then
	export PATH=$HOME/.local/bin:$PATH
fi

#zprof

[ -f ~/.fzf.zsh ] && source ~/.fzf.zsh

# Set QT theme
export QT_QPA_PLATFORMTHEME="qt6ct"

# if command -v wal >/dev/null; then
# 	(cat ~/.cache/wal/sequences &)
# fi

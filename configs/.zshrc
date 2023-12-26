# Enable history
setopt histignorespace
setopt histignoredups
setopt sharehistory
setopt incappendhistory

HISTSIZE=100000
SAVEHIST=10000
HISTFILE=${HOME}/.zsh_history

# Enable auto completion
setopt auto_menu
setopt auto_list
setopt auto_param_keys
setopt auto_param_slash
setopt auto_remove_slash
setopt autocd
setopt auto_pushd

zstyle :compinstall filename '${HOME}/.zshrc'
autoload -Uz compinit && compinit
zstyle ':completion:*' menu select
# zstyle ':completion:*' matcher-list 'm:{a-z}={A-Za-z}'

# Theme
if command -v starship >/dev/null; then
	export STARSHIP_CONFIG=${HOME}/.config/starship.toml
	eval "$(starship init zsh)"
fi

# Enable syntax highlighting
source ${HOME}/.config/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh

# Enable auto suggestions
source ${HOME}/.config/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh

# Enable FZF
source ${HOME}/.config/zsh/plugins/fzf-zsh-plugin/fzf-zsh-plugin.plugin.zsh

# Enable zsh f-sy-h
source ${HOME}/.config/zsh/plugins/F-Sy-H/F-Sy-H.plugin.zsh

# zsh - z
source ${HOME}/.config/zsh/plugins/zsh-z/zsh-z.plugin.zsh

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
bindkey -M viins "^P" up-history
bindkey -M viins "^N" down-history

# Cargo environment
if command -v rustup >/dev/null; then
	if [ -f ${HOME}/.cargo/env ]; then
		source ${HOME}/.cargo/env
	else
		export PATH="${HOME}/.cargo/bin:${PATH}"
	fi
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

autoload -Uz edit-command-line
zle -N edit-command-line
bindkey '^X^E' edit-command-line
fpath+=${ZDOTDIR:-${HOME}}/.zsh_functions

if command -v nvim >/dev/null 2>&1; then
	export EDITOR=nvim
fi

if command -v broot >/dev/null 2>&1; then
	source ${HOME}/.config/broot/launcher/bash/br
fi

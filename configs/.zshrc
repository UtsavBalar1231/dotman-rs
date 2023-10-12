# Enable history
setopt histignorespace
setopt histignoredups
setopt sharehistory
setopt incappendhistory

HISTSIZE=10000
SAVEHIST=10000
HISTFILE=~/.zsh_history

# Enable auto completion
setopt auto_menu
setopt auto_list
setopt auto_param_keys
setopt auto_param_slash
setopt auto_remove_slash

# Enable auto cd
setopt auto_cd

# Enable auto pushd
setopt auto_pushd

# fast typing
xset r rate 250 100

zstyle :compinstall filename '~/.zshrc'
autoload -Uz compinit && compinit
zstyle ':completion:*' menu select
# zstyle ':completion:*' matcher-list 'm:{a-z}={A-Za-z}'

# Theme
export STARSHIP_CONFIG=~/.config/starship.toml
eval "$(starship init zsh)"

# Enable syntax highlighting
source ~/.config/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh

# Enable auto suggestions
source ~/.config/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh

# Enable FZF
source ~/.config/zsh/plugins/fzf-zsh-plugin/fzf-zsh-plugin.plugin.zsh

# Enable zsh f-sy-h
source ~/.config/zsh/plugins/F-Sy-H/F-Sy-H.plugin.zsh

# zsh - z
source ~/.config/zsh/plugins/zsh-z/zsh-z.plugin.zsh

# Enable aliases
setopt aliases
source ~/.config/zsh/aliases.zsh

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
if [ -f ~/.cargo/env ]; then
	source ~/.cargo/env
fi

if [ -d ~/.local/bin/ ]; then
	export PATH="${HOME}/.local/bin":${PATH}
fi

# Gitlint
if [ -f ~/.gitlint ]; then
	GITLINT_CONFIG=~/.gitlint
	export GITLINT_CONFIG
fi

if [ -d /usr/local/go ]; then
	export GOARCH=amd64
	export GOOS=linux
	export GOROOT=/usr/local/go
	export PATH=$GOROOT/bin:$PATH
fi

autoload -Uz edit-command-line
zle -N edit-command-line
bindkey '^X^E' edit-command-line
fpath+=${ZDOTDIR:-~}/.zsh_functions

if command -v nvim >/dev/null 2>&1; then
	export EDITOR=nvim
fi

if command -v broot >/dev/null 2>&1; then
	source ~/.config/broot/launcher/bash/br
fi

export ACE_INSTALL_DIR=/home/vicharak/hdd/achronix/ACE/Achronix-linux
export PATH=${PATH}:/home/vicharak/hdd/achronix/SNPS/linux64/bin

export EFINITY_HOME=$HOME/hdd/shreeyash/efinity/2023.1/
export EFXPT_HOME=$EFINITY_HOME/pt
export PYTHON_PATH=$EFINITY_HOME/bin
export PATH=$PYTHON_PATH:$EFINITY_HOME/scripts:$PATH

export DEBFULLNAME="UtsavBalar1231"
export DEBEMAIL="utsavbalar1231@gmail.com"

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

# Enable syntax highlighting
source ~/.config/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh

# Enable auto suggestions
source ~/.config/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh

# Enable FZF
source ~/.config/zsh/plugins/fzf-zsh-plugin/fzf-zsh-plugin.plugin.zsh

# Enable forgit
#source ~/.config/zsh/plugins/forgit/forgit.plugin.zsh

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

# Theme
source ~/.config/zsh/theme/cunt-theme.zsh-theme

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

alias ssh="kitty +kitten ssh"

if [ -d /usr/local/go ]; then
	export GOARCH=amd64
	export GOOS=linux
	export GOROOT=/usr/local/go
	export PATH=$GOROOT/bin:$PATH
fi


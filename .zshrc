HISTFILE=~/.histfile
HISTSIZE=10000
SAVEHIST=10000
HISTDUP=erase
setopt appendhistory
setopt sharehistory
setopt incappendhistory
setopt hist_ignore_all_dups
setopt hist_save_no_dups
setopt hist_ignore_dups
setopt hist_find_no_dups

bindkey -e

zstyle :compinstall filename '/home/utsav/.zshrc'

autoload -Uz compinit && compinit
zstyle ':completion:*' matcher-list 'm:{a-z}={A-Za-z}'

# Alias definitions.
source ~/.config/zsh/aliases.zsh

# User configuration
export EDITOR=nvim

# language environment
export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

# zsh syntax highlighting
if [[ ! -f $HOME/.config/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh ]]; then
	git clone --depth=1 https://github.com/zsh-users/zsh-syntax-highlighting.git $HOME/.config/zsh/plugins/zsh-syntax-highlighting
fi
source $HOME/.config/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh

# zsh autosuggestions
if [[ ! -f $HOME/.config/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh ]]; then
	git clone --depth=1 https://github.com/zsh-users/zsh-autosuggestions $HOME/.config/zsh/plugins/zsh-autosuggestions
fi
source $HOME/.config/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh

# zsh completion
if [[ ! -f $HOME/.config/zsh/plugins/zsh-completions/zsh-completions.plugin.zsh ]]; then
	git clone --depth=1 https://github.com/zsh-users/zsh-completions $HOME/.config/zsh/plugins/zsh-completions
fi
fpath=($HOME/.config/zsh/plugins/zsh-completions/src $fpath)
autoload -Uz compinit && compinit

# zsh history search
if [[ ! -f $HOME/.config/zsh/plugins/zsh-history-substring-search/zsh-history-substring-search.zsh ]]; then
	git clone --depth=1 https://github.com/zsh-users/zsh-history-substring-search $HOME/.config/zsh/plugins/zsh-history-substring-search
fi
source $HOME/.config/zsh/plugins/zsh-history-substring-search/zsh-history-substring-search.zsh

# zsh theme
source $HOME/.config/zsh/theme/cunt-theme.zsh-theme

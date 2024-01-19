#/usr/bin/env zsh

# alias for ls
eza_params=('--git' '--icons' '--classify' '--group-directories-first' '--time-style=long-iso' '--group' '--color-scale')

alias ls='eza $eza_params'
alias l='eza --git-ignore $eza_params'
alias ll='eza --all --header --long $eza_params'
alias llm='eza --all --header --long --sort=modified $eza_params'
alias la='eza -lbhHigUmuSa'
alias lx='eza -lbhHigUmuSa@'
alias lt='eza --tree $eza_params'
alias tree='eza --tree $eza_params'

# alias for bat
alias b='bat'

# alias for rg
alias rg='rg --smart-case'
alias rgf='rg --files'
alias rgd='rg --files-with-matches'

if command -v fdfind >/dev/null; then
	alias fd='fdfind'
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

alias alacritty="wmctrl -x -a "tabbed" || tabbed alacritty --embed&"

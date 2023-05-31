#/usr/bin/env zsh

# alias for ls
alias ls='exa'
alias ll='exa -l -g --icons'
alias la='exa -la -g --icons'
alias l='exa -l'

# alias for bat
alias b='bat'

# alias for rg
alias rg='rg --smart-case'
alias rgf='rg --files'
alias rgd='rg --files-with-matches'
alias grep='rg'

alias n='nvim'
alias v='vim'

# git aliases
alias gc='git clone'
alias gs='git status'
alias gd='git diff'
alias gdc='git diff --cached'
alias gch='git checkout'
alias gco='git commit'
alias gca='git commit --amend'
alias gcp='git cherry-pick'
alias gpl='git pull'
alias gps='git push'
alias gbr='git branch'
alias grst='git reset'
alias grsth='git reset --hard'
alias grb='git rebase'
alias gl='git log'
alias gl1='git log --oneline'
alias gr='git revert'
alias grm='git remote'
alias gcl='git clean'

# find using fzf and edit
alias ffe='fzf-find-edit'

alias sudo='sudo '

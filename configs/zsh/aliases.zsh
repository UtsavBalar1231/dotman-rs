#/usr/bin/env zsh

# alias for ls
alias ls='eza'
alias ll='eza -l -g --icons'
alias la='eza -la -g --icons'
alias l='eza -l'

# alias for bat
alias b='bat'

# alias for rg
alias rg='rg --smart-case'
alias rgf='rg --files'
alias rgd='rg --files-with-matches'
alias fd='fdfind'

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

# find using fzf and edit
alias ffe='fzf-find-edit'

alias sudo='sudo '
alias tvmc='python3 -m tvm.driver.tvmc'

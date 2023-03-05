#/usr/bin/env zsh

alias ls='exa'
alias ll='exa -l -g --icons'
alias la='exa -la -g --icons'
alias l='exa -l'

alias grep='rg'
alias rg='rg --smart-case'
alias rgf='rg --files'
alias rgd='rg --files-with-matches'

alias n='nvim'
alias v='nvim'

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

# find using fzf and edit
alias ffe='fzf-find-edit'

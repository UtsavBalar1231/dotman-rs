#!/usr/bin/env bash

### Setup zsh as default shell

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

if ! command -v zsh &>/dev/null; then
    echo "zsh not installed"
    exit 1
fi

# Set zsh as default for current user
chsh -s "$(which zsh)"

# Set zsh as default for root user
sudo chsh -s "$(which zsh)"
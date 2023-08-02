#!/usr/bin/env bash

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

cp -f "$(dirname "${CUR_DIR}")"/.gitconfig ~/

# Setup githooks

if [ ! -d ~/.git/hooks ]; then
	mkdir -p ~/.git/hooks
fi

git config --global core.hooksPath ~/.git/hooks

curl -sLo ~/.git/hooks/commit-msg https://gist.githubusercontent.com/UtsavBalar1231/c48cb6993ff45b077d41c13622fc27ba/raw/66f7da7f128a9511df81d624f23f87fc294b59b6/commit-msg

if [ ! -f ~/.git/hooks/commit-msg ]; then
	echo "Failed to download commit-msg hook"
	exit 1
fi

chmod u+x ~/.git/hooks/commit-msg

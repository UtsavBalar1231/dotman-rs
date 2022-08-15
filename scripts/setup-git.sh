#!/usr/bin/env bash

cp $(pwd)/../.gitconfig ~/

git config --global user.name UtsavBalar1231
git config --global user.email utsavbalar1231@gmail.com
git config --global credential.helper store
git config --global merge.log 10000

# Setup githooks
mkdir -p ~/.git/hooks
git config --global core.hooksPath ~/.git/hooks
curl -sLo ~/.git/hooks/commit-msg https://gist.githubusercontent.com/UtsavBalar1231/c48cb6993ff45b077d41c13622fc27ba/raw/66f7da7f128a9511df81d624f23f87fc294b59b6/commit-msg
chmod u+x ~/.git/hooks/commit-msg

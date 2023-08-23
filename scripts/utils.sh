#!/usr/bin/env bash

get_git_version() {
	local git_package="${1}"

	curl --silent "https://api.github.com/repos/${git_package}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

get_ubuntu_version() {
	lsb_release -ds | cut -d ' ' -f 2 | cut -d '.' -f 1
}

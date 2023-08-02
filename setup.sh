#!/usr/bin/env bash

set -euo pipefail

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

# Set local timezone
export TZ="Asia/Kolkata"

# Setup build environment: {{{
bash "${CUR_DIR}"/scripts/setup_git.sh
bash "${CUR_DIR}"/scripts/setup_env.sh
# }}}

# Install rust: {{{
if ! command -v rustup &>/dev/null; then
	bash "${CUR_DIR}"/scripts/setup_rust.sh
fi
# }}}

get_git_version() {
	local git_package="${1}"

	curl --silent "https://api.github.com/repos/${git_package}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

# Install diff-so-fancy: {{{
if ! command -v diff-so-fancy &>/dev/null; then
	diff_so_fancy_version=$(get_git_version "so-fancy/diff-so-fancy")

	curl -sLo ./diff-so-fancy https://github.com/so-fancy/diff-so-fancy/releases/download/"${diff_so_fancy_version}"/diff-so-fancy

	chmod a+x ./diff-so-fancy

	sudo mv ./diff-so-fancy /usr/local/bin/diff-so-fancy
fi
# }}}

usage() {
	echo "Usage: ${0} [OPTIONS]"
	echo "Options:"
	echo "  arch: setup Arch Linux"
	echo "  ubuntu: setup Ubuntu"
	echo "  help: show this help message"
}

if echo "${@}" | grep -wqE "help|-h"; then
	if [ -n "${2}" ] && [ "$(type -t usage"${2}")" == function ]; then
		echo "--- ${2} Setup Commands ---"
		eval usage "${2}"
	else
		usage
	fi
	exit 0
fi

OPTIONS=("${@}")
for option in "${OPTIONS[@]}"; do
	echo "processing option: $option"
	case ${option} in
		*arch)
			./setup-arch.sh
			;;
		*ubuntu|*debian)
			./setup-linux.sh
			;;
		help|-h)
			usage
			;;
		*)
			echo "unknown option: $option"
			usage
			;;
	esac
done

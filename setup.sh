#!/usr/bin/env bash

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

# shellcheck disable=SC1090
source "${CUR_DIR}"/scripts/utils.sh

# Set local timezone
export TZ="Asia/Kolkata"

# Setup build environment: {{{
echo "Setting up git..."
source "${CUR_DIR}"/scripts/setup_git.sh
echo "Setting up environment..."
source "${CUR_DIR}"/scripts/setup_env.sh
# }}}

# Install rust: {{{
if ! command -v rustup &>/dev/null; then
	echo "Setting up rust..."
	source "${CUR_DIR}"/scripts/setup_rust.sh
fi
# }}}

# Install diff-so-fancy: {{{
if ! command -v diff-so-fancy &>/dev/null; then
	echo "Setting up diff-so-fancy..."
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
		echo "Setting up Arch..."
		./setup-arch.sh
		;;
	*ubuntu | *debian)
		echo "Setting up Ubuntu/Debian..."
		./setup-linux.sh
		;;
	help | -h)
		usage
		;;
	*)
		echo "unknown option: $option"
		usage
		;;
	esac
done

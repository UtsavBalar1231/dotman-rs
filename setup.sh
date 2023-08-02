#!/usr/bin/env bash

set -euo pipefail

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

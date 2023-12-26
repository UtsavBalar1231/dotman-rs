#!/usr/bin/env bash

touch /tmp/i3-startup.log

if ! pgrep -u "$USER" ssh-agent >/dev/null; then
	ssh-agent -t 1h >"$XDG_RUNTIME_DIR/ssh-agent.env"
fi
if [[ ! -f "$SSH_AUTH_SOCK" ]]; then
	# shellcheck disable=SC1091
	source "$XDG_RUNTIME_DIR/ssh-agent.env" >/dev/null
fi

# Auto set monitor
available_monitors="$(xrandr -q | grep -w connected | awk '{print $1}' | wc -l)"
if [[ $available_monitors -eq 2 ]]; then
	autorandr dual-monitors
fi

# make keyboard smooth
xset r rate 250 120

# Set the background
if [[ -f "${HOME}"/.config/i3/wallpaper-slideshow.sh ]]; then
	bash "${HOME}"/.config/i3/wallpaper-slideshow.sh &
	disown
fi

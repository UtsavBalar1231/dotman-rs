#!/usr/bin/env bash

DEBUG=true

function log() {
	if ${DEBUG}; then
		echo "$@"
	fi
}

LOG_FILE=/tmp/i3-startup.log
if [[ -f "$LOG_FILE" ]]; then
	rm -f "$LOG_FILE"
fi
touch ${LOG_FILE}

# Start ssh-agent
if ! pgrep -u "$USER" ssh-agent >/dev/null; then
	ssh-agent -t 1h >"$XDG_RUNTIME_DIR/ssh-agent.env"

	log "Started ssh-agent" >> ${LOG_FILE}
fi
if [[ ! -f "$SSH_AUTH_SOCK" ]]; then
	# shellcheck disable=SC1091
	source "$XDG_RUNTIME_DIR/ssh-agent.env" >/dev/null
fi

# Start dunst
if ! pgrep -u "$USER" dunst >/dev/null; then
	dunst &

	log "Started dunst" >> ${LOG_FILE}
fi

# Auto set monitor
available_monitors="$(xrandr -q | grep -w connected | awk '{print $1}' | wc -l)"
if [[ $available_monitors -eq 2 ]]; then
	autorandr dual-monitors

	log "Started autorandr dual-monitors" >> ${LOG_FILE}
fi

# make keyboard smooth
xset r rate 250 120
log "set Keyboard repeat rate 250 120" >> ${LOG_FILE}

# Set the background
if [[ -f "${HOME}"/.config/i3/wallpaper-slideshow.sh ]]; then
	bash "${HOME}"/.config/i3/wallpaper-slideshow.sh &
	disown

	log "Started wallpaper slideshow" >> ${LOG_FILE}
fi

# Start polkit agent
if [[ -f /usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1 ]]; then
	/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1 &
	disown

	log "Started polkit-gnome-authentication-agent-1" >> ${LOG_FILE}
elif [[ -f /usr/lib/polkit-kde/polkit-kde-authentication-agent-1 ]]; then
	/usr/lib/polkit-kde/polkit-kde-authentication-agent-1 &
	disown

	log "Started polkit-kde-authentication-agent-1" >> ${LOG_FILE}
elif command -v "polkit-dumb-agent" &>/dev/null; then
	polkit-dumb-agent &
	disown

	log "Started polkit-dumb-agent" >> ${LOG_FILE}
elif command -v "lxqt-policykit-agent" &>/dev/null; then
	lxqt-policykit-agent &
	disown

	log "Started lxqt-policykit-agent" >> ${LOG_FILE}
fi

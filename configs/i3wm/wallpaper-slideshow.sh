#!/usr/bin/env bash

# Set the background
if [ -d "${HOME}/Pictures/wallpapers/" ]; then
	# check XDG session
	session=$(echo "$XDG_SESSION_TYPE" | tr '[:upper:]' '[:lower:]')

	case "$session" in
		"x11")
			while true; do
				feh --randomize --no-fehbg --bg-max "${HOME}/Pictures/wallpapers/" | tee -a /tmp/i3-startup.log
				sleep 60
			done
			;;
		"wayland")
			while true; do
				img=$(find "${HOME}/Pictures/wallpapers/" | shuf -n 1)
				swaybg -m fill -i "${img}"
				sleep 60
			done
			;;
		*)
			while true; do
				feh --randomize --no-fehbg --bg-max "${HOME}/Pictures/wallpapers/" | tee -a /tmp/i3-startup.log
				sleep 60
			done
			;;
		esac
fi

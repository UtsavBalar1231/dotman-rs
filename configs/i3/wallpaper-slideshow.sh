#!/usr/bin/env bash

# Set the background
if [ -d "${HOME}/Pictures/wallpapers/" ]; then
	while true; do
		feh --randomize --no-fehbg --bg-max "${HOME}/Pictures/wallpapers/" | tee -a /tmp/i3-startup.log
		sleep 60
	done
fi

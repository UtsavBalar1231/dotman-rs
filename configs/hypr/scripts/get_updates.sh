#!/bin/bash

# Get the number of AUR updates available
pacman_updates=$(pacman -Qu | wc -l)
if command -v aur >/dev/null 2>&1; then
	aur_updates=$(pacman -Qm | aur vercmp | wc -l)
	total_updates=$((pacman_updates + aur_updates))
else
	total_updates=$pacman_updates
fi
tooltip="There are $total_updates updates available."
output="{\"text\": \"$total_updates\", \"tooltip\": \"$tooltip\"}"
echo "$output" | jq --unbuffered --compact-output

#!/bin/bash

if [ -f ~/.cache/perfmode ] ;then
    hyprctl reload
    rm ~/.cache/perfmode
    notify-send "󰀨 perfmode deactivated" "Animations and blur enabled!"
else
    hyprctl --batch "\
        keyword animations:enabled 0;\
        keyword decoration:drop_shadow 0;\
        keyword decoration:blur:enabled 0;\
        keyword general:gaps_in 0;\
        keyword general:gaps_out 0;\
        keyword general:border_size 1;\
        keyword decoration:rounding 0"
	touch ~/.cache/perfmode
    notify-send "󰀨 perfmode activated" "Animations and blur disabled!"
fi

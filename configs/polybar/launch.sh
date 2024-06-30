#!/usr/bin/env bash

# Terminate already running bar instances
# If all your bars have ipc enabled, you can use
polybar-msg cmd quit
# Otherwise you can use the nuclear option:
killall -q polybar

# Wait until the processes have been shut down
while pgrep -u $UID -x polybar >/dev/null; do sleep 1; done

# Launch bar1 and bar2
available_monitors="$(xrandr -q | grep -w connected | awk '{print $1}')"

for monitor in $available_monitors; do
	case $monitor in
	"HDMI-1-0")
		polybar hdmi-1-0 2>&1 | tee -a /tmp/polybar1.log &
		disown
		;;
	"eDP1")
		polybar edp1 2>&1 | tee -a /tmp/polybar1.log &
		disown
		;;
	*)
		polybar main 2>&1 | tee -a /tmp/polybar1.log &
		disown
		;;
	esac
done

echo "Bars launched..."

#!/bin/bash

# Function to get network details
get_network_details() {
	# Icons for network
	local icon_wifi=" "
	local icon_ethernet="󰈀 "
	local icon_disconnected="󰖪 "

	# Variables for status
	local wifi_status=""
	local ethernet_status=""

	# Get network interface information
	local interfaces
	interfaces=$(ip link show | awk '/^[0-9]+: / {print $2}')

	for interface in $interfaces; do
		interface=${interface%:} # Remove the trailing colon
		local ip_address
		ip_address=$(ip addr show "$interface" | awk '/inet / {print $2}' | cut -d/ -f1)

		if [[ -n "$ip_address" ]]; then
			if [[ "$interface" =~ ^w ]]; then
				essid=$(iw dev "$interface" link | awk '/SSID/ {for (i=2; i<=NF; i++) printf "%s ", $i; print ""}')
				# Wi-Fi interface
				wifi_status="$icon_wifi $essid"
			elif [[ "$interface" =~ ^e ]]; then
				# Ethernet interface
				ethernet_status="$icon_ethernet $ip_address"
			fi
		fi
	done

	# Determine final output
	if [[ -n "$wifi_status" && -n "$ethernet_status" ]]; then
		# Both Wi-Fi and Ethernet connected
		echo "$wifi_status $ethernet_status"
	elif [[ -n "$wifi_status" ]]; then
		# Only Wi-Fi connected
		echo "$wifi_status"
	elif [[ -n "$ethernet_status" ]]; then
		# Only Ethernet connected
		echo "$icon_disconnected $ethernet_status"
	else
		# Neither Wi-Fi nor Ethernet connected
		echo "$icon_disconnected"
	fi
}


# Function to get battery percentage
get_battery_percentage() {
	local battery_percentage=-1
	if [[ -f /sys/class/power_supply/BAT1/capacity ]]; then
		battery_percentage=$(cat /sys/class/power_supply/BAT1/capacity)
	else
		echo "󰂎 Error: Battery information not found."
		exit 1
	fi

	local battery_icons=(
		"󰂎" "󰁺" "󰁻" "󰁼" "󰁽"
		"󰁾" "󰁿" "󰂀" "󰂁" "󰂂"
		"󰁹"
	)

	if [[ $battery_percentage -ge 0 ]]; then
		echo "${battery_icons[$((battery_percentage / 10))]} $battery_percentage%"
	else
		echo "󰂎 No battery found."
		exit 1
	fi
}

# Function to get the current user
get_whoami() {
	local user_icon=" "
	whoami=$(whoami)
	echo "$user_icon $whoami"
}

# Function to get the current playing song
get_which_song() {
	local icon_playing="󰎈 "
	local icon_paused="󰏤 "
	local icon_resumed="󰐊 "

	# Get music status and current song
	local music_status
	local which_song
	music_status=$(mpc status | awk '/^\[.*\]/ {print $1}')
	which_song=$(mpc current)

	# Determine icon based on music status
	case "$music_status" in
		"[playing]")
			echo "$icon_playing $which_song"
			;;
		"[paused]")
			echo "$icon_paused $which_song"
			;;
		"[stopped]")
			echo "$icon_resumed Not Playing"
			;;
		*)
			echo ""
			;;
	esac
}

# Main case logic
case "$1" in
	"network")
		get_network_details
		;;
	"battery")
		get_battery_percentage
		;;
	"whoami")
		get_whoami
		;;
	"song")
		get_which_song
		;;
	*)
		echo "󰇚 Usage: $0 {network|battery|whoami|song}"
		exit 1
		;;
esac

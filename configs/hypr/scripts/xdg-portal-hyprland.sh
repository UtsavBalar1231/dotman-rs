#!/bin/bash

# Kill all possible running xdg-desktop-portals
killall -e xdg-desktop-portal-hyprland
killall -e xdg-desktop-portal-gnome
killall -e xdg-desktop-portal-kde
killall -e xdg-desktop-portal-lxqt
killall -e xdg-desktop-portal-wlr
killall -e xdg-desktop-portal-gtk
killall -e xdg-desktop-portal

/usr/lib/xdg-desktop-portal-hyprland &
sleep 0.5

# Start xdg-desktop-portal-gtk
if [ -f /usr/lib/xdg-desktop-portal-gtk ] ;then
    /usr/lib/xdg-desktop-portal-gtk &
    sleep 0.5
fi

/usr/lib/xdg-desktop-portal &
sleep 0.5

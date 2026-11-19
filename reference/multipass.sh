#!/bin/bash
# MultiPass fuer Linux starten.
# Falls das Fenster nicht aufgeht: Tkinter nachinstallieren, z. B.
#   Debian/Ubuntu:  sudo apt install python3-tk
#   Fedora:         sudo dnf install python3-tkinter
#   Arch:           sudo pacman -S tk
cd "$(dirname "$0")" || exit 1
exec python3 multipass.py "$@"

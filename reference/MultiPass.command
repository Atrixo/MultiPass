#!/bin/bash
# MultiPass fuer macOS starten (Doppelklick im Finder).
cd "$(dirname "$0")" || exit 1
if command -v python3 >/dev/null 2>&1; then
  exec python3 multipass.py "$@"
else
  osascript -e 'display dialog "Python 3 wurde nicht gefunden. Bitte von python.org installieren." buttons {"OK"}'
fi

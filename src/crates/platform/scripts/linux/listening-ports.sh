#!/bin/sh
set -eu
exec /usr/bin/lsof -nP -a -p "$1" -iTCP -sTCP:LISTEN -Fn

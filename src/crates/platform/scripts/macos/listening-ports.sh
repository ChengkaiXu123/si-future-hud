#!/bin/sh
set -eu
exec /usr/sbin/lsof -nP -a -p "$1" -iTCP -sTCP:LISTEN -Fn

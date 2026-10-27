#!/bin/sh
set -eu
# xdg-open accepts a directory; use a quoted positional path, never eval.
exec xdg-open "$(dirname -- "$1")"

#!/bin/sh
set -eu
exec /bin/ps -U "$1" -o pid=,comm=

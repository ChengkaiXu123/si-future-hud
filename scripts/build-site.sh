#!/bin/sh
# Assembles the product site for GitLab Pages into public/: the page and its
# assets from site/, and the loops and stills the README uses from
# docs/si-future-hud/media (as media/). Used by .gitlab-ci.yml and for a
# local preview (open public/index.html).
set -eu
cd "$(dirname "$0")/.."
rm -rf public
mkdir -p public/media
cp -R site/. public/
cp docs/si-future-hud/media/*.webp docs/si-future-hud/media/*.jpg public/media/
echo "public/: $(find public -type f | wc -l | tr -d ' ') files, $(du -sh public | cut -f1)"

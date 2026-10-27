#!/bin/zsh
# Carries main's new commits to the company GitLab.
#
# GitHub (origin) keeps main as it is, with the anonymous GitHub author. The
# company GitLab only takes commits by a registered member, so its copy, the
# local branch gitlab-main, has the same changes with the author
# Chengkai Xu <ckxu@trip.com>. refs/sync/gitlab-main (local only) remembers
# the last main commit carried over.
#
#   git push origin main && scripts/sync-gitlab.sh
set -euo pipefail
cd "$(dirname "$0")/.."
marker=refs/sync/gitlab-main
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Commit or stash your changes first." >&2
  exit 1
fi
from=$(git rev-parse --verify "$marker")
commits=($(git rev-list --reverse "$from..main"))
if (( ${#commits} == 0 )); then
  echo "GitLab already carries main up to $(git rev-parse --short main)."
  exit 0
fi
start=$(git symbolic-ref --short -q HEAD || git rev-parse HEAD)
export GIT_AUTHOR_NAME="Chengkai Xu" GIT_AUTHOR_EMAIL="ckxu@trip.com"
export GIT_COMMITTER_NAME="Chengkai Xu" GIT_COMMITTER_EMAIL="ckxu@trip.com"
git switch -q gitlab-main
for c in $commits; do
  git cherry-pick --allow-empty --keep-redundant-commits "$c" >/dev/null
  git commit --amend --no-edit --reset-author -q
done
git update-ref "$marker" main
git switch -q "$start"
git push gitlab gitlab-main:main
echo "GitLab now carries main up to $(git rev-parse --short main) (${#commits} new commit(s))."

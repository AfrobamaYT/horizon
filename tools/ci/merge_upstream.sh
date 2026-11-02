#!/usr/bin/env bash
# Exit 0: merged; 2: conflicts (left for the caller to report); 3: already contained.
set -euo pipefail

if git merge-base --is-ancestor upstream/main HEAD; then
  echo 'Upstream main is already contained.'
  exit 3
fi

export GIT_AUTHOR_NAME=arismoko GIT_COMMITTER_NAME=arismoko
export GIT_AUTHOR_EMAIL=19542195+arismoko@users.noreply.github.com
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"
sha=$(git rev-parse --short upstream/main)
if git -c commit.gpgsign=false merge --no-ff -m "Merge upstream main $sha" upstream/main; then
  exit 0
fi

if [ -n "$(git diff --name-only --diff-filter=U)" ]; then
  exit 2
fi
exit 1

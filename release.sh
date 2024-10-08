#!/bin/bash

GITHUB_UPSTREAM_URL="https://github.com/ston-fi/tonlib-rs.git"
GITHUB_UPSTREAM_NAME="github_tonlib_rs"

if [ "$(git symbolic-ref --short HEAD)" != "main" ]; then
    echo "release.sh works only from main branch"
    exit 1
fi

git pull origin main

DEFAULT_NEW_VERSION="$(cat Cargo.toml | grep "version" | head -n 1 | rev | awk -F\" '{print $2}' | rev)"
NEW_VERSION=""
read -p "Enter new version (if empty: ${DEFAULT_NEW_VERSION}): " NEW_VERSION
if [ "${NEW_VERSION}" =  "" ]; then
    NEW_VERSION="${DEFAULT_NEW_VERSION}"
fi
echo new_version: ${NEW_VERSION}

RELEASE_BRANCH_NAME="upstream-${NEW_VERSION}"
git checkout -b ${RELEASE_BRANCH_NAME}


git config remote.${GITHUB_UPSTREAM_NAME}.url >&- || git remote add ${GITHUB_UPSTREAM_NAME} ${GITHUB_UPSTREAM_URL}
git fetch ${GITHUB_UPSTREAM_NAME} main
git merge ${GITHUB_UPSTREAM_NAME}/main

echo "Upstream diff:"
git diff ${GITHUB_UPSTREAM_NAME}/main

should_proceed=""
read -p "Push changes to ${GITHUB_UPSTREAM_URL}? (yes/no): " should_proceed
if [ "${should_proceed}" != "yes" ]; then
    echo "Release stopped by user."
    echo "To push the branch: git push ${GITHUB_UPSTREAM_NAME} ${RELEASE_BRANCH_NAME}"
    exit 0
fi

git push ${GITHUB_UPSTREAM_NAME} ${RELEASE_BRANCH_NAME}

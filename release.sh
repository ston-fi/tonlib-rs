#!/bin/bash

if [ "$(git symbolic-ref --short HEAD)" != "main" ]; then
    echo "release.sh works only from main branch"
    exit 1
fi

if [ "$(git diff --stat)" != "" -a "${ALLOW_DIRTY}" != "1" ]; then
    echo "Repo is dirty: please cleanup or set ALLOW_DIRTY=1"
    exit 1
fi

GITHUB_UPSTREAM_URL="https://github.com/ston-fi/tonlib-rs.git"
GITHUB_UPSTREAM_NAME="github_tonlib_rs"

git pull origin main

# ===== handle version =====
CURRENT_VERSION="$(cat Cargo.toml | grep "^version" | head -n 1 | rev | awk -F\" '{print $2}' | rev | sed 's/+.*//')"
DEFAULT_NEW_VERSION="$(echo ${CURRENT_VERSION} | awk -F. -v OFS=. 'NF==1{print ++$NF}; NF>1{if(length($NF+1)>length($NF))$(NF-1)++; $NF=sprintf("%0*d", length($NF), ($NF+1)%(10^length($NF))); print}')"
NEW_VERSION=""
echo ""
echo "CURRENT_VERSION: ${CURRENT_VERSION}"
read -p "Enter NEW version (if empty: ${DEFAULT_NEW_VERSION}): " NEW_VERSION
if [ "${NEW_VERSION}" =  "" ]; then
    NEW_VERSION="${DEFAULT_NEW_VERSION}"
fi
echo NEW_VERSION: ${NEW_VERSION}

# ===== handle changelog & release commit =====
CHANGELOG="$(printf "### v${NEW_VERSION}\n$(git rev-list --format=%B --no-merges --ancestry-path v${CURRENT_VERSION}..HEAD | grep -v "commit" | grep -v '^[[:space:]]*$' | awk '{print "* " toupper(substr($0,0,1))tolower(substr($0,2))}')")"
echo ""
echo "Changelog:"
echo "${CHANGELOG}"

sed -i '' "s/^version =.*/version = \"${NEW_VERSION}\"/" Cargo.toml
echo "${CHANGELOG}" >> CHANGELOG.md
git diff

should_proceed=""
read -p "Wanna edit changelog? (y/n): " should_proceed
if [ "${should_proceed}" = "y" ]; then
    vim CHANGELOG.md
fi

should_proceed=""
read -p "Should I create release commit with such changes? (y/n): " should_proceed
if [ "${should_proceed}" != "y" ]; then
    echo "Release stopped by user."
    exit 0
fi

git commit -am "Release v${NEW_VERSION}"
git tag v${NEW_VERSION}
git push origin main
git push origin v${NEW_VERSION}


RELEASE_BRANCH_NAME="upstream-${NEW_VERSION}"
git checkout -b ${RELEASE_BRANCH_NAME}

git config remote.${GITHUB_UPSTREAM_NAME}.url >&- || git remote add ${GITHUB_UPSTREAM_NAME} ${GITHUB_UPSTREAM_URL}
git fetch ${GITHUB_UPSTREAM_NAME} main
git merge ${GITHUB_UPSTREAM_NAME}/main
git tag v${NEW_VERSION}

echo "Upstream diff:"
git diff ${GITHUB_UPSTREAM_NAME}/main

should_proceed=""
read -p "Push changes to ${GITHUB_UPSTREAM_URL}? (y/n): " should_proceed
if [ "${should_proceed}" != "y" ]; then
    echo "Release stopped by user."
    echo "To push the branch: git push ${GITHUB_UPSTREAM_NAME} ${RELEASE_BRANCH_NAME}"
    exit 0
fi

git push ${GITHUB_UPSTREAM_NAME} ${RELEASE_BRANCH_NAME}

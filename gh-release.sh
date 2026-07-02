#!/bin/sh
repo=$(git remote get-url origin | sed 's/.*:\(.*\)\.git/\1/')
name=$(basename "$repo")
url="https://github.com/$repo/releases/download/latest/$name.so"
# curl -L -O "$url"
echo $url

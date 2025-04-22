#!/bin/bash

STATUS_FILE=$(mktemp)
PATCH_FILE=../musl.patch
NEW_FILES=(src/glib/glib.c)

error() {
    echo "$@"
    exit 1
}

# Moves to musl subdirectory, if not already there
if [ "$(basename $PWD)" != "musl" ]; then
    cd musl || error Unable to locate musl subdirectory
fi

for file in "${NEW_FILES[@]}"; do
    git add -N "$file"
done

# Compares the current unstaged changes with the patch
git diff >$STATUS_FILE
diff $STATUS_FILE $PATCH_FILE &>/dev/null

if [ "$?" != 0 ]; then
    # If changes are different than the patch, apply it if there are no changes, otherwise show an error message
    if [ ! -s "$STATUS_FILE" ]; then
        echo Applying patch to musl
        git apply $PATCH_FILE
        exit 0
    fi

    echo -e "\e[33mUnable to apply musl patch due to conflicting changes in the musl subdirectory\e[0m"
else
    # Do nothing if changes are already present
    echo Musl patch already applied, skipping
fi

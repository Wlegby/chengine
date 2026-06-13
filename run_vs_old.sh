#!/bin/bash

DATE=$(date +%d-%m-%Y_%H-%M)

# 1. Directory Setup
ARCHIVE_DIR="./engines_archive"
GAME_ARCHIVE_DIR="./game_archive"

# Ensure the game archive directory exists
mkdir -p "$GAME_ARCHIVE_DIR"

# 2. Find the highest version number in the archive
LAST_VERSION=$(ls "$ARCHIVE_DIR" | grep -E '^chengine-v[0-9]+$' | sed 's/chengine-v//' | sort -n | tail -n 1)

# Safety check: Make sure we actually found a version greater than 1
if [[ -z "$LAST_VERSION" || "$LAST_VERSION" -le 1 ]]; then
  echo "❌ Need at least version 2 to play against older versions. Exiting."
  exit 1
fi

NEW_ENGINE_NAME="chengine-v$LAST_VERSION"
NEW_ENGINE_PATH="$ARCHIVE_DIR/$NEW_ENGINE_NAME"

echo "⚔️ Prepping gauntlet: $NEW_ENGINE_NAME VS older versions"
echo "-------------------------------------------------------------"

# 3. Build the Cutechess arguments dynamically
CUTECHESS_ARGS=(
  "-tournament" "gauntlet"
  "-engine" "cmd=$NEW_ENGINE_PATH" "name=$NEW_ENGINE_NAME"
)

# 4. Loop through all older versions (from 1 up to LAST_VERSION - 1)
for ((i = 1; i < LAST_VERSION; i++)); do
  OPP_NAME="chengine-v$i"
  OPP_PATH="$ARCHIVE_DIR/$OPP_NAME"

  # Only add the engine if the file actually exists and is executable
  if [[ -x "$OPP_PATH" ]]; then
    CUTECHESS_ARGS+=("-engine" "cmd=$OPP_PATH" "name=$OPP_NAME")
    echo "Added opponent: $OPP_NAME"
  else
    echo "⚠️ Warning: $OPP_PATH not found or not executable. Skipping."
  fi
done

# 5. Add the match constraints and UNLIMITED time controls
# Choose ONE of the options below inside the array:
CUTECHESS_ARGS+=(
  "-each" "tc=inf"
  "-rounds" "4"
  "-concurrency" "6"
  "-pgnout" "$GAME_ARCHIVE_DIR/games_${NEW_ENGINE_NAME}_vs_older_${DATE}.pgn"
)

echo "-------------------------------------------------------------"
echo "Starting Cutechess..."

# 6. Run Cutechess using the array of arguments
cutechess-cli "${CUTECHESS_ARGS[@]}"

echo "Matches finished!"

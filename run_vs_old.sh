#!/bin/bash

STOCKFISH_PATH="/usr/sbin/stockfish" # Change to your stockfish path

# 1. Directory Setup
ARCHIVE_DIR="./engines_archive"
BOOK_DIR="polyglot.bin"
mkdir -p "$ARCHIVE_DIR"
mkdir -p "$BOOK_DIR"

# 2. Find the next sequential version number (e.g., version_4)
LAST_VERSION=$(ls "$ARCHIVE_DIR" | grep -E '^chengine-v[0-9]+$' | sed 's/chengine-v//' | sort -n | tail -n 1)

if [ -z "$LAST_VERSION" ]; then
  NEXT_VERSION=1
else
  NEXT_VERSION=$((LAST_VERSION + 1))
fi

NEW_ENGINE_NAME="chengine-v$NEXT_VERSION"
NEW_ENGINE_PATH="$ARCHIVE_DIR/$NEW_ENGINE_NAME"

# 5. Determine the opponent (the previous version)
if [ "$NEXT_VERSION" -eq 1 ]; then
  echo "⚠️ This is chengine-v1. No older versions exist yet to play against."
  echo "💡 Exiting. Run the script again after modifying your code to create version_2!"
  exit 0
fi

PREV_VERSION=$((NEXT_VERSION - 1))
OLD_ENGINE_PATH="$ARCHIVE_DIR/chengine-v$PREV_VERSION"

echo "⚔️ Prepping gauntlet: $NEW_ENGINE_NAME VS chengine-v$PREV_VERSION"
echo "-------------------------------------------------------------"

# 6. Run the games simultaneously using fastchess
# (Adjust concurrency to your CPU cores and tc for time control)
fastchess \
  -engine cmd="$NEW_ENGINE_PATH" name="$NEW_ENGINE_NAME" \
  -engine cmd="$STOCKFISH_PATH" name="SF_1500" option.UCI_LimitStrength=true option.UCI_Elo=1800 \
  -engine cmd="$STOCKFISH_PATH" name="SF_2000" option.UCI_LimitStrength=true option.UCI_Elo=2000 \
  -engine cmd="$STOCKFISH_PATH" name="SF_2300" option.UCI_LimitStrength=true option.UCI_Elo=2300 \
  -engine cmd="$STOCKFISH_PATH" name="SF_2500" option.UCI_LimitStrength=true option.UCI_Elo=2500 \
  -each tc=5+0.05 \
  -book file="$BOOK_DIR" format=polyglot plies=5 \
  -rounds 40 \
  -concurrency 8 \
  -pgnout file="games_${NEW_ENGINE_NAME}_vs_stockfish.pgn"

echo "🏁 Match complete. PGN saved to games_${NEW_ENGINE_NAME}_vs_v${PREV_VERSION}.pgn"

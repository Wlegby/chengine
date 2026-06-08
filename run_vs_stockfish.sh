#!/bin/bash

STOCKFISH_PATH="/usr/sbin/stockfish" # Change to your stockfish path

# 1. Directory Setup
ARCHIVE_DIR="./engines_archive"
BOOK_DIR="polyglot.bin"

# 2. Find the next sequential version number (e.g., version_4)
LAST_VERSION=$(ls "$ARCHIVE_DIR" | grep -E '^chengine-v[0-9]+$' | sed 's/chengine-v//' | sort -n | tail -n 1)

NEW_ENGINE_NAME="chengine-v$LAST_VERSION"
NEW_ENGINE_PATH="$ARCHIVE_DIR/$NEW_ENGINE_NAME"

echo "⚔️ Prepping gauntlet: $NEW_ENGINE_NAME VS stockfish"
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

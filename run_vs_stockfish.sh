#!/bin/bash

STOCKFISH_PATH="/usr/sbin/stockfish" # Change to your stockfish path
DATE=$(date +%d-%m-%Y_%H-%M)

# 1. Directory Setup
ARCHIVE_DIR="./engines_archive"

# 2. Find the next sequential version number (e.g., version_4)
LAST_VERSION=$(ls "$ARCHIVE_DIR" | grep -E '^chengine-v[0-9]+$' | sed 's/chengine-v//' | sort -n | tail -n 1)

ENGINE_NAME="chengine-v$LAST_VERSION"
ENGINE_PATH="$ARCHIVE_DIR/$NEW_ENGINE_NAME"

echo "⚔️ Prepping gauntlet: $NEW_ENGINE_NAME VS stockfish"
echo "-------------------------------------------------------------"

# 6. Run the games simultaneously using fastchess
# (Adjust concurrency to your CPU cores and tc for time control)
fastchess \
  -tournament gauntlet \
  -engine cmd="$ENGINE_PATH" name="$ENGINE_NAME" st=5000 \
  -engine cmd="$STOCKFISH_PATH" name="SF_1800" option.UCI_LimitStrength=true option.UCI_Elo=1800 st=5 \
  -engine cmd="$STOCKFISH_PATH" name="SF_2000" option.UCI_LimitStrength=true option.UCI_Elo=2000 st=5 \
  -engine cmd="$STOCKFISH_PATH" name="SF_2200" option.UCI_LimitStrength=true option.UCI_Elo=2200 st=5 \
  -engine cmd="$STOCKFISH_PATH" name="SF_2500" option.UCI_LimitStrength=true option.UCI_Elo=2500 st=5 \
  -rounds 2 \
  -concurrency 6 \
  -pgnout file="game_archive/games_${NEW_ENGINE_NAME}_vs_stockfish_gauntlet_${DATE}.pgn"

echo "Matches running"

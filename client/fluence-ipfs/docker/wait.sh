#!/usr/bin/env bash
set -euo pipefail

CMD1=$1        # Take first arg as a command  
CMD2=$2        # Take second arg as a command
                
eval "$CMD1&"  # Run in background
eval "$CMD2&"  # Run in background

wait -n        # Wait for first terminated child, return its exit code
echo "One command failed, killing all children"
pkill -P $$   # Kill all children, printing kill command for each one

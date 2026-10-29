#!/bin/sh
# nav.sh <from state> <to state> <prefix> <frames after each> tap|btn args...
# each step: "x,y" taps the bottom screen, "B:NAME" presses a button,
# "W:n" waits n frames; a screenshot is taken after each step.
from=$1; to=$2; prefix=$3; wait=$4; shift 4
cd /home/pmill/icarus/trace
script=$(mktemp)
echo "load $from" > "$script"
i=0
for step in "$@"; do
  i=$((i+1))
  case $step in
    B:*) echo "hold ${step#B:}" >> "$script"; echo "run 8" >> "$script"; echo "hold" >> "$script" ;;
    W:*) echo "run ${step#W:}" >> "$script" ;;
    *) echo "touch $(echo "$step" | tr , ' ')" >> "$script"; echo "run 6" >> "$script"; echo "untouch" >> "$script" ;;
  esac
  echo "run $wait" >> "$script"
  echo "shot $prefix$i" >> "$script"
done
echo "save $to" >> "$script"
/home/pmill/icarus/tools/kiutrace/target/release/kiutrace ../dumps/kiu.cci "$script" out | grep -v "^\[" | tail -3
python3 -I /home/pmill/icarus/scripts/sheet.py out "$prefix" "$i" "out/sheet_$prefix.png"
rm "$script"

#!/bin/sh
# Imports code.bin into a headless Ghidra project as ARM11 code loaded at
# 0x00100000, runs auto-analysis, then the Icarus scripts.
#
# usage: ghidra/analyze.sh [ghidra install dir] [code.bin]
set -e
here=$(cd "$(dirname "$0")" && pwd)
ghidra=${1:-$here/../tools/ghidra_12.2_PUBLIC}
export JAVA_HOME=${JAVA_HOME:-$(ls -d "$here"/../tools/jdk-25*)}
export PATH="$JAVA_HOME/bin:$PATH"
code=${2:-$here/../dumps/exefs/code.bin}
mkdir -p "$here/project" "$here/reports"

"$ghidra/support/analyzeHeadless" "$here/project" kiu \
    -import "$code" -overwrite \
    -loader BinaryLoader \
    -loader-baseAddr 0x00100000 \
    -processor ARM:LE:32:v6 \
    -scriptPath "$here" \
    -preScript SetupCodeBin.java \
    -postScript FindProjection.java "$here/reports/projection.txt" \
    -analysisTimeoutPerFile 7200

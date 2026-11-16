#!/bin/bash
# shot.sh VARIANT WIDTH HEIGHT OUT URLSUFFIX [JS-wait-condition]   (URLSUFFIX e.g. "?live=bbbbbbbb2#tok")
S=$(dirname "$0"); V=$1; W=$2; H=$3; OUT=$4; SUF=${5:-"#tok"}; COND=${6:-"document.body.innerText.includes('Live') && !!document.querySelector('.live-session')"}; PORT=$((8800 + RANDOM % 100))
python3 $S/fixture_server.py $PORT /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/T095/src/easy_personal/dashboard/static $V > /dev/null 2>&1 &
SP=$!; sleep 1
node $S/cdp_shot.mjs "http://127.0.0.1:$PORT/$SUF" $W $H $OUT "$COND"
kill $SP

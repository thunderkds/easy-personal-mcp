#!/bin/bash
# shot.sh VARIANT WIDTH HEIGHT OUT [JS-wait-condition]
S=$(dirname "$0"); V=$1; W=$2; H=$3; OUT=$4; COND=${5:-"!!document.querySelector('.status-chips') && !document.body.innerText.includes('Loading usage')"}; PORT=$((8800 + RANDOM % 100))
python3 $S/fixture_server.py $PORT /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/T068/src/easy_personal/dashboard/static $V > $S/srv.log 2>&1 &
SP=$!; sleep 1
node $S/cdp_shot.mjs "http://127.0.0.1:$PORT/#tok" $W $H $OUT "$COND"
kill $SP

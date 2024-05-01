#!/bin/sh
BOXES=~/Projects/boxes
source $BOXES/venv/bin/activate
$BOXES/scripts/boxes Rack19Box \
    --output=rack-leds-box-3mm.svg --format=svg \
	--FingerJoint_space='4.0' \
    --thickness=3 --burn=0 --reference=0 \
    --depth=30 --height=1 --d1=5 --d2=5 --triangle=15

# 442w 30d 34.5h inner
$BOXES/scripts/boxes TrayInsert \
    --output=rack-leds-insert-3mm.svg --format=svg \
    --thickness=3 --burn=0 --reference=0 \
    --sx='10 422 10' --sy='10 20' --h='30'

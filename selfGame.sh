#!/bin/bash

cargo build --release
./halite -q -d "30 30" "target/release/MyBot" "target/release/MyBotPrevious"

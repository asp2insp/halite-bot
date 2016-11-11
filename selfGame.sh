#!/bin/bash

cargo build --release
./halite -d "30 30" "target/release/MyBot" "target/release/MyBotPrevious"

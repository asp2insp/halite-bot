set -x
f="A2Ibot.zip"
if [ -e "$f" ]
then
  rm "$f"
fi

zip "$f" Cargo.toml src/MyBot.rs src/hlt/* 

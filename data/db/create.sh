#!/bin/bash

if [[ -f "bot.db" ]]; then
    cp bot.db bot.db.bak
fi

last=-1

if [[ -f "last" ]]; then
    last=$(cat last)
fi

echo "Last run migration was $last"

for file in ./*.sql
do
    num=${file%%-*}
    num=${num##./}
    if (( $num <= $last )); then
        echo "Skipping $file"
        continue
    fi
    echo "Running $file on bot.db"
    sqlite3 bot.db < "$file"
done

echo "Done updating"
echo $num > last
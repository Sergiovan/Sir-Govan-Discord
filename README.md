# GOVAN IS BACK (AGAIN)
In Rust, for a challenge (and because ts was taking too much memory). This is just an administrative/silly fun bot for my private server.

# Functionality
## No context
One in 100 messages that are less than 261 characters are put in the #no-context channel. You'll also get a randomized role!

## Reactions

### Hall of fame
If any one message gets 3 📌 reactions from people who are not the author, it enters the hall of fame for posterity. 

### Other halls
If a message gets 3 😶 reactions, it'll be sent to the hall of things with mysterious energies. 

Messages with 3 😩 reactions get sent to the hall of people who just cannot spell right.

Messages with 3 of _any other reaction without a use_ get sent to a chaos hall, where everything is chaos.

### The twitterverse
Reacting with 🔁 or 🔂 on a message sends it to the Infinitely Tall Cylinder Earth version of twitter and returns a picture of your message as a tweet there.
🔂 only takes your text, and 🔁 also takes other people's messages as extra tweets below yours.

### Never sunny in here
Reacting with 🎻 on a message makes it into a short titlecard video with the music of IASIP. For the niche comedic value that brings every now and then.

### This is the dark souls of features
Reacting with ❤️‍🔥 on a message makes it into a small dark souls themed banner image. Reacting with 🪦 makes it into a banner in the style of the famous "You Died" message. 

## Commands
### `!pin {message}`
Puts a message in the hall of fame. But it'll be marked. Cheaters beware, democracy is strong.

### `!color [hex or 'random']`
Changes your role color if any of your roles have a color. Because I can't be arsed with administration. You can also randomize your color.

### `!icon {emoji or url}`
Changes your role icon if in a server that allows that to happen, and if you're not trying to trick the bot

### `!role`
Gives out the number and the name of the current randomized role. Collect them all!

### `!roll [sides]`
Rolls a D[sides] or a D20 if no sides are given. Warning: Highly addictive

### `!ping`
Pong!

### MORE
There's way more commands but they're for admin control, or just not that interesting to put here.

# Building
Just `cargo build` :). You might need a gcc compiler to get damn Ring to work properly. 

~~For a raspberry pi, check out these cool cross-compilers: https://github.com/tttapa/docker-arm-cross-toolchain. You could, theoretically, also use https://github.com/cross-rs/cross but I don't know how that works.~~

Nevermind, cross compiling skia-safe is impossible, it cannot be done. Give up. 

# FAQ

## Why did you make this?
This is the spiritual successor to my Steam bot Sir Govan. Just for fun and no profit.

## Can you add \<IDEA>?
If I like it and you're not rude, maybe.

## Why is there no !help?
It's not needed. If you want help, just scroll up. Or read the ` c o d e `, this is open source, we don't do documentation. 

## I hate this
That's not a question.

## I hate this?
The fact I've made this joke in every FAQ, or this bot?


"use strict";

import { Bot } from './bot';
import { discord } from './secrets';
import { cmds, aliases, beta_cmds } from './commands';
import { listeners } from './listeners';

let bot = new Bot(discord.token, process.argv.includes('--beta'));

for (let cmd in cmds) {
    bot.addCommand(`!${cmd}`, cmds[cmd]);
}

for (let alias in aliases) {
    bot.addCommand(`!${alias}`, aliases[alias]);
}

if (bot.beta) {
    for (let cmd in beta_cmds) {
        bot.addCommand(`!${cmd}`, beta_cmds[cmd]);
    }
}

for (let event in listeners) {
    bot.setEventListener(event, listeners[event]);
}

bot.connect();
"use strict";

import Eris from 'eris';
import * as fs from 'fs';
import * as path from 'path';
import * as util from 'util';

import { Bot } from './bot';

import { botparams, argType, emojis } from './defines';
import { arg, parseArgs, randomBigInt, rb_ } from './utils';

export type CommandFunc = (msg: Eris.Message) => void;

export const cmds: { [key: string]: CommandFunc } = {
    die(this: Bot, msg: Eris.Message) {
        if (msg.author.id === botparams.owner) {
            this.die();
        }
    },

    ping(this: Bot, msg: Eris.Message) {
        this.reply(msg, "Pong", this.text.ping);
    },

    pingDM(this: Bot, msg: Eris.Message) {
        this.replyDM(msg, "Pong", this.text.ping);
    },

    roll(this: Bot, msg: Eris.Message) {
        let [num] = parseArgs(msg, arg(argType.bigint, 20n));
        this.reply(msg, `${randomBigInt(num, 1n)}${rb_(this.text.roll, "")}`);
    },

    color(this: Bot, msg: Eris.Message) {
        let server = botparams.servers.getServer(msg);
        if (!server) {
            return;
        }
        let [color] = parseArgs(msg, arg(argType.string, `${Math.floor(Math.random() * 0x1000000)}`));
        if (!color) {
            this.reply(msg, "Missing color parameter. Please provide me with a hex number", this.text.colorNoColor);
            return;
        }
        let number = Number.parseInt('0x' + (''+color).replace(/^(#|0x)/, ''));
        if (!Number.isNaN(number) && Number.isFinite(number) && number >= 0 && number < 0x1000000) {
            let member = msg.member;
            let guild = (msg.channel as Eris.TextChannel).guild;
            let roles = guild.roles;
            let user_roles = roles.filter((role) => member?.roles.includes(role.id) || false);
            user_roles.sort((a, b) => b.position - a.position);
            user_roles[0].edit({color: number});
        } else if (Number.isNaN(number)) {
            this.reply(msg, "That's not a valid color hex. Give me a valid hex, like #C0FFEE or #A156F2", this.text.colorNaN);
        } else if (!Number.isFinite(number)) {
            this.reply(msg, "That color would blow your mind. Give me a valid hex, like #0084CA or #F93822", this.text.colorInfinite);
        } else if (number < 0) {
            this.reply(msg, "A negative hex? Now I know you're just fucking with me", this.text.colorNegative);
        } else {
            this.reply(msg, "I'm unsure your monitor can even render that. Your hex is too high. " +
                "Give me a valid hex, like #00AE8F, or #F2F0A1", this.text.colorError);
        }
    },

    role(this: Bot, msg: Eris.Message) {
        let server = botparams.servers.getServer(msg);
        if (!server) {
            return;
        }
        let self = this;
        if (server.no_context_role) {
            let rolename = (msg.channel as Eris.TextChannel).guild.roles.get(server.no_context_role)?.name;
            if (!rolename) {
                console.log(`The no context role ${server.no_context_role} doesn't exist in ${server.id}`.red);
                return;
            }
            fs.readFile(path.join('data', 'nocontext.txt'), "utf8", function(err, data) {
                let index = -1;
                let total = 0;
                if (err) {
                    console.log(`Error detected: ${err}`);
                } else {
                    let lines = data.trim().split('\n');
                    index = lines.indexOf(rolename!);
                    total = lines.length;
                }
                rolename += index === -1 ? "\nNote: This role does not exist anymore. It's a shiny!" : "";
                let index_str = index === -1 ? 'NaN' : `${index+1}/${total}`;
                self.client.createMessage(msg.channel.id, `${index_str}: ${rolename}`);
            });
        } else {
            // Very niche, sleep well
            this.reply(msg, "This server does not have roles to collect", this.text.roleRoleNotAvailable);
        }
    },

    async pin(this: Bot, msg: Eris.Message) { 
        let server = botparams.servers.getServer(msg);
        if (!server) {
            return;
        }
        let self = this;
        let thischannel = msg.channel;
        let [messageID] = parseArgs(msg, arg(argType.string));
        if (messageID) {
            let success = false;
            for (let elem of (msg.channel as Eris.TextChannel).guild.channels) {
                let [_, channel] = elem;
                if (server.allowed_channels_listen.includes(channel.id)) {
                    try {
                        let msg = await (channel as Eris.TextChannel).getMessage(messageID as string);
                        if (msg.reactions[emojis.pushpin.fullName] && msg.reactions[emojis.pushpin.fullName].me) {
                            self.client.createMessage(thischannel.id, rb_(self.text.pinAlreadyPinned, "That message is already pinned"));
                            return;
                        }
                        msg.addReaction(emojis.pushpin.fullName);
                        self.pin(msg, true);
                        success = true;
                        break;
                    } catch (err) {
                        
                    }
                }
            }
            if (!success) {
                this.reply(msg, "Invalid message ID", this.text.pinInvalidMessage);
            }
        } else {
            this.reply(msg, "Missing message ID", this.text.pinNoMessage);
        }
    },

    puzzle(this: Bot, msg: Eris.Message) {
        msg.channel.createMessage(this.puzzleHelp());
    },

    puzzle_pause(this: Bot, msg: Eris.Message) {
        if (msg.author.id === botparams.owner) {
            if (this.puzzler.togglePaused()) {
                msg.channel.createMessage(`Puzzle has been stopped`);
            } else {
                msg.channel.createMessage(`Puzzle has been resumed`);
            }
        }
    },

    check(this: Bot, msg: Eris.Message) {
        let [answer] = parseArgs(msg, arg(argType.string));
        if (answer) {
            if (this.puzzler.checkAnswer(answer as string)) {
                this.reply(msg, "Correct!", this.text.checkCorrect);
            } else {
                this.reply(msg, "Incorrect", this.text.checkIncorrect);
            }
        } else {
            this.reply(msg, "Missing answer to check", this.text.checkMissingAnswer);
        }

    },

    async reload_text(this: Bot, msg: Eris.Message) {
        if (msg.author.id === botparams.owner) {
            let [res, err] = await this.loadText();
            if (res) {
                this.reply(msg, "Text reloaded successfully");
            } else {
                this.replyDM(msg, `Error loading text: \`\`\`${err}\`\`\``);
            }
        }
    }
};

export const aliases: { [key: string]: CommandFunc } = {
    colour: cmds.color
}

export const beta_cmds: { [key: string]: CommandFunc} = {
    debug(this: Bot, msg: Eris.Message) {
        console.log(util.inspect(msg, true, 5, true));
    },
    __die: cmds.die,
    post_clue(this: Bot, msg: Eris.Message) {
        let server = botparams.servers.getServer(msg);
        if (!server || server.beta !== this.beta || msg.author.id !== botparams.owner) {
            return;
        }
        this.postClue(server.allowed(msg) ? msg.channel.id : server.allowed_channels[0], true);
    }
}
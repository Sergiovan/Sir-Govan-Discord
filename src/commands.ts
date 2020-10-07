"use strict";

import Eris from 'eris';
import * as fs from 'fs';
import * as path from 'path';
import * as util from 'util';

import { Bot } from './bot';

import { botparams, argType, emojis } from './defines';
import { arg, parseArgs, randomBigInt, randFromFile } from './utils';

export type CommandFunc = (msg: Eris.Message) => void;

export const cmds: { [key: string]: CommandFunc } = {
    die(this: Bot, msg: Eris.Message) {
        if (msg.author.id === botparams.owner) {
            this.die();
        }
    },

    roll(this: Bot, msg: Eris.Message){
        let [err, num] = parseArgs(msg, arg(0, '20'));
        let bignum: bigint;
        if (!err) {
            try {
                bignum = BigInt(num);
            } catch (e) {
                bignum = 20n;
            }
            this.client.createMessage(msg.channel.id, `${randomBigInt(bignum, 1n)}`);
        }
    },

    color(this: Bot, msg: Eris.Message){
        let server = botparams.servers.getServer(msg);
        if (!server) {
            return;
        }
        let [err, color] = parseArgs(msg, arg(argType.string, `${Math.floor(Math.random() * 0x1000000)}`));
        if (!err) {
            let number = Number.parseInt('0x' + (''+color).replace(/^(#|0x)/, ''));
            if (!Number.isNaN(number) && Number.isFinite(number) && number >= 0 && number < 0x1000000) {
                let member = msg.member;
                let guild = (msg.channel as Eris.TextChannel).guild;
                let roles = guild.roles;
                let user_roles = roles.filter((role) => member?.roles.includes(role.id) || false);
                user_roles.sort((a, b) => b.position - a.position);
                user_roles[0].edit({color: number});
            } else if (Number.isNaN(number)) {
                this.client.createMessage(msg.channel.id, "That's not a valid color hex. Give me a valid hex, like #C0FFEE or #A156F2");
            } else if (!Number.isFinite(number)) {
               this.client.createMessage(msg.channel.id, "That color would blow your mind. Give me a valid hex, like #0084CA or #F93822");
            } else if (number < 0) {
                this.client.createMessage(msg.channel.id, "A negative hex? Now I know you're just fucking with me");
            } else {
                this.client.createMessage(msg.channel.id, "I'm unsure your monitor can even render that. Your hex is too high. " +
                    "Give me a valid hex, like #00AE8F, or #F2F0A1");
            }
        } else {
            this.client.createMessage(msg.channel.id, "Incredibly, something went wrong. I will now tell my master about it");
            console.log('Something went very wrong when changing colors :<'.red);
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
            this.client.createMessage(msg.channel.id, "This server does not have roles to collect. Sorry!");
        }
    },

    async pin(this: Bot, msg: Eris.Message) { 
        let server = botparams.servers.getServer(msg);
        if (!server) {
            return;
        }
        let self = this;
        let thischannel = msg.channel;
        let [err, messageID] = parseArgs(msg, arg(argType.string));
        if (!err) {
            if (messageID) {
                for (let elem of (msg.channel as Eris.TextChannel).guild.channels) {
                    let [_, channel] = elem;
                    if (server.allowed_channels_listen.includes(channel.id)) {
                        try {
                            let msg = await (channel as Eris.TextChannel).getMessage(messageID as string);
                            if (msg.reactions[emojis.pushpin.fullName] && msg.reactions[emojis.pushpin.fullName].me) {
                                self.client.createMessage(thischannel.id, "I already pinned that message >:(");
                                return;
                            }
                            msg.addReaction(emojis.pushpin.fullName);
                            self.pin(msg, true);
                        } catch (err) {
                            console.log(`Message not in ${channel.name}: ${err}`);
                        }
                    }
                }
            } else {
                this.client.createMessage(msg.channel.id, "You're gonna have to give me a message ID, pal");
            }
        } else {
            this.client.createMessage(msg.channel.id, "Hm. Something went wrong there");
        }
    },

    puzzle(this: Bot, msg: Eris.Message) {
        msg.channel.createMessage(this.puzzleHelp());
    },

    puzzle_pause(this: Bot, msg: Eris.Message) {
        if (msg.author.id === botparams.owner) {
            this.puzzle_stopped = !this.puzzle_stopped;
            if (this.puzzle_stopped) {
                msg.channel.createMessage(`Puzzle has been stopped`);
            } else {
                msg.channel.createMessage(`Puzzle has been resumed`);
            }
        }
    },

    check(this: Bot, msg: Eris.Message) {
        let [err, answer] = parseArgs(msg, arg(argType.string));
        if (!err) {
            if (answer === this.answer) {
                this.client.createMessage(msg.channel.id, "That's the one!");
            } else {
                this.client.createMessage(msg.channel.id, "This ain't it chief");
            }
        } else {
            this.client.createMessage(msg.channel.id, "Please give me something to check :(");
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
    __die: cmds.die
}
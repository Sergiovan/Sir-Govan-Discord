import * as D from 'discord.js';
import * as fs from 'fs';
import * as path from 'path';
import * as util from 'util';

import { Bot } from './bot';

import { botparams, argType, emojis } from '../defines';
import { arg, parseArgs, randomBigInt, rb_, Logger } from '../utils';

export type CommandFunc = (msg: D.Message) => void;

/** Stores all commands as functions */
// TODO :)
export const cmds: { [key: string]: CommandFunc } = {
    /** Kills the bot. Only I can use it though, no nonsense */
    die(this: Bot, msg: D.Message) {
        if (msg.author.id === botparams.owner) {
            this.die();
        }
    },

    /** Debug command to check the status of the bot */
    status(this: Bot, msg: D.Message) {
        if (msg.author.id === botparams.owner) {
            Logger.inspect(util.inspect(this, true, 3, true));
        }
    },

    /** Obligatory ping command */
    ping(this: Bot, msg: D.Message) {
        this.reply(msg, "Pong", this.text.ping);
    },

    /** Ping but it sends you a DM */
    pingDM(this: Bot, msg: D.Message) {
        this.replyDM(msg, "Pong", this.text.ping);
    },

    /** Rolls an n-sided dice */
    roll(this: Bot, msg: D.Message) {
        let [num] = parseArgs(msg, arg(argType.bigint, 20n));
        this.reply(msg, `${randomBigInt(num, 1n)}${rb_(this.text.roll, "")}`);
    },

    /** Changes a user's unique role color */
    color(this: Bot, msg: D.Message) {
        let [color] = parseArgs(msg, arg(argType.string, `${Math.floor(Math.random() * 0x1000000)}`));
        if (!color) {
            this.reply(msg, "Missing color parameter. Please provide me with a hex number", this.text.colorNoColor);
            return;
        }
        let number = Number.parseInt('0x' + (''+color).replace(/^(#|0x)/, ''));
        if (!Number.isNaN(number) && Number.isFinite(number) && number >= 0 && number < 0x1000000) {
            if (!msg.member) {
                return; // Not in a server
            }
            let member = msg.member;
            let guild = (msg.channel as D.TextChannel).guild;

            let user_roles = member.roles.cache.filter(role => !!role.color);
            user_roles.sort((a, b) => b.position - a.position);

            // Edit the first unique role
            for (let [role_id, role] of user_roles) {
                if (guild.members.cache.find((usr) => usr.id !== member.id && usr.roles.cache.has(role_id))) {
                    continue;
                }
                role.setColor(number);
                return;
            }
            this.reply(msg, "It seems like you have no valid role to color");
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

    /** Gives the current no-context role in use */
    role(this: Bot, msg: D.Message) {
        let server = botparams.servers.getServer(msg);
        if (!msg.guild || !server) {
            return;
        }
        if (server.no_context_role) {
            let rolename = msg.guild.roles.resolve(server.no_context_role)?.name;
            if (!rolename) {
                Logger.warning(`The no context role ${server.no_context_role} doesn't exist in ${server.id}`);
                return;
            }
            let self = this;
            fs.readFile(path.join('data', 'nocontext.txt'), "utf8", function(err, data) {
                let index = -1;
                let total = 0;
                if (err) {
                    Logger.error(`Error detected: ${err}`);
                } else {
                    let lines = data.trim().split('\n');
                    index = lines.indexOf(rolename!);
                    total = lines.length;
                }
                // TODO Textify shiny reply
                rolename += index === -1 ? "\nNote: This role does not exist anymore. It's a shiny!" : "";
                let index_str = index === -1 ? 'NaN' : `${index+1}/${total}`;
                msg.channel.send(`${index_str}: ${rolename}`);
            });
        } else {
            // Very niche, sleep well
            this.reply(msg, "This server does not have roles to collect", this.text.roleRoleNotAvailable);
        }
    },

    /** Forcefully pin something into hall-of-fame */
    async pin(this: Bot, msg: D.Message) { 
        let server = botparams.servers.getServer(msg);
        if (!msg.guild || !server) {
            return; // Invalid
        }
        if (!server.pin_channel) {
            // TODO reply something funny?
            return;
        }
        let self = this;
        let thischannel = msg.channel; // This must be an allowed channel already
        let [messageID] = parseArgs(msg, arg(argType.string));
        if (messageID) {
            let success = false;
            for (let [channel_id, channel] of msg.guild.channels.cache) {
                if (channel.isText() && server.allowed_channels_listen.includes(channel_id)) { // The message given is in a channel the bot can listen to
                    try {
                        let msg = await channel.messages.fetch(messageID as D.Snowflake);
                        if (msg.reactions.resolve(emojis.pushpin.toString())?.me) {
                            thischannel.send(rb_(self.text.pinAlreadyPinned, "That message is already pinned"));
                            return;
                        }
                        msg.react(emojis.pushpin.toString());
                        await self.pin(msg, true);
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

    /** Gives help and info relating to the puzzle */
    puzzle(this: Bot, msg: D.Message) {
        this.reply(msg, this.puzzleHelp());
    },

    /** Pauses the puzzle, meaning no new clues will drop */
    puzzle_pause(this: Bot, msg: D.Message) {
        if (msg.author.id === botparams.owner) {
            if (this.puzzler.togglePaused()) {
                msg.channel.send(`Puzzle has been stopped`);
            } else {
                msg.channel.send(`Puzzle has been resumed`);
            }
        }
    },

    /** Checks if the argument is the solution to the puzzle without actually using it */
    check(this: Bot, msg: D.Message) {
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

    /** Reloads the text instanct of the bot. Useful if the text is changed while the bot is still running */
    async reload_text(this: Bot, msg: D.Message) {
        if (msg.author.id === botparams.owner) {
            let [res, err] = await this.loadText();
            if (res) {
                this.reply(msg, "Text reloaded successfully");
            } else {
                this.replyDM(msg, `Error loading text: \`\`\`${err}\`\`\``);
            }
        }
    },

    /** ??? */
    // steal(this: Bot, msg: D.Message) {
    //     let first_reply = rb_(this.text.stealFirst, '');
    //     if (first_reply.length) {
    //         this.reply(msg, first_reply);
    //     }
    // }
};

/** Holds all aliasts, that is, commands that point to other commands. Really should standardize this tbh */
export const aliases: { [key: string]: CommandFunc } = {
    colour: cmds.color
}

/** Holds all commands that can only be ran if the bot is in beta mode */
export const beta_cmds: { [key: string]: CommandFunc} = {
    /** Gives debug info for the message that calls this command */
    debug(this: Bot, msg: D.Message) {
        Logger.inspect(util.inspect(msg, true, 3, true));
    },

    /** Alias of die, effectively only kills beta bot */
    __die: cmds.die,

    /** Forces a clue to be posted */
    post_clue(this: Bot, msg: D.Message) {
        let server = botparams.servers.getServer(msg);
        if (!server || server.beta !== this.beta || msg.author.id !== botparams.owner || !server.puzzle_channel) {
            return;
        }
        this.postClue(server.puzzle_channel, true);
    }
}
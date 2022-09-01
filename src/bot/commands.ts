import * as D from 'discord.js';
import * as fs from 'fs';
import * as path from 'path';
import * as util from 'util';

import { Bot } from './bot';

import { argType, Emoji, emojis, emoji_url, regexes, Server } from '../defines';
import { arg, Arg, parseArgs, randomBigInt, rb_, Logger, parseArgsHelper } from '../utils';

export type CommandFunc = (msg: D.Message) => void;

/** Stores all commands as functions */
// TODO :)
export const cmds: { [key: string]: CommandFunc } = {
    /** Kills the bot. Only I can use it though, no nonsense */
    die(this: Bot, msg: D.Message) {
        if (msg.author.id === this.ownerID) {
            this.die();
        }
    },

    /** Debug command to check the status of the bot */
    status(this: Bot, msg: D.Message) {
        if (msg.author.id === this.ownerID) {
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
        if (num as bigint <= 0) {
            this.reply(msg, "Dice need to have 1 or more sides, otherwise I don't know where the North is", this.text.roll_no_sides);
            return;
        }
        let number = `${randomBigInt(num as bigint, 1n)}`.replace(/(\d)(?=(\d{3})+(?!\d))/g, "$1,"); // Good god https://stackoverflow.com/a/25377176
        this.reply(msg, `${number}${rb_(this.text.roll, "")}`);
    },

    /** Changes a user's unique role color */
    color(this: Bot, msg: D.Message) {
        let server = this.get_server(msg);
        if (!msg.guild || !server) return;

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
            let role = this.get_member_role(member);

            if (role) {
                role.setColor(number);
            } else {
                this.reply(msg, "It seems like you have no valid role to color");
            }
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

    /** Changes your role icon, if possible */
    async icon(this: Bot, msg: D.Message) {
        let server = this.get_server(msg);
        if (!msg.guild || !msg.member || !server) return;

        let [icon] = parseArgs(msg, arg(argType.string, null, true));

        if (msg.guild.premiumTier === D.GuildPremiumTier.None || msg.guild.premiumTier === D.GuildPremiumTier.Tier1) {
            this.reply(msg, "This server cannot actually change role icons, sorry!");
            return;
        }

        let member = msg.member;
        let role = this.get_member_role(member);

        if (!role) {
            this.reply(msg, "It seems you don't have a valid role to get an icon on");
            return;
        }

        if (!icon) {
            try {
                role.setIcon(null);
                this.reply(msg, "Your icon has been removed!");
            } catch (err) {
                Logger.inspect(err);
                this.reply(msg, "Something went wrong! Go bug my master to see");
            }
        } else {
            try {
                let emoji = regexes.discord_emojis.exec(icon);
                if (emoji && emoji[3]) {

                    let emoji_obj = role.guild.emojis.resolve(emoji[3]);

                    if (!emoji_obj || emoji_obj.guild.id !== msg.guild.id) {
                        // this.reply(msg, "The emoji needs to be from this server");
                        let foreign_emoji = emoji_url(emoji[3]);
                        await role.setIcon(foreign_emoji);
                        return;
                    }
                    await role.setIcon(emoji[3]); // Emoji snowflake
                    this.reply(msg, `Your icon has been set: ${icon}!`);
                } else if (icon.startsWith('http://') || icon.startsWith('https://')) {
                    await role.setIcon(icon); // url?
                    this.reply(msg, "Your icon has been set!");
                } else {
                    try {
                        await role.setUnicodeEmoji(icon);
                    } catch (err) {
                        Logger.inspect(err);
                        this.reply(msg, "You must provide a valid image url, discord emoji or unicode emoji!");
                    }
                }
            } catch (err: any) {
                Logger.inspect(err);
                Logger.inspect(err.cause);
                for (let prop in err) {
                    Logger.debug(prop);
                    Logger.inspect(err[prop])
                }
                switch (err?.rawError?.errors?.icon?._errors[0]?.code ?? err?.cause?.code ?? err.code) {
                    case 'ERR_INVALID_URL':
                        this.reply(msg, "That's not a valid url, my jolly friend");
                        break;
                    case 'ENOTFOUND':
                        this.reply(msg, "I cannot find anything at that location...");
                        break;
                    case 'IMAGE_INVALID':
                        this.reply(msg, "That's not an image, is it");
                        break;
                    case 'BINARY_TYPE_MAX_SIZE':
                        this.reply(msg, "This image is too powerful for Discord to handle. Try finding something smaller");
                        break;
                    default:
                        this.reply(msg, "Well, I don't even know what happened there. Go bother my master about it");
                        break;
                }
            }
        }

    },

    /** Gives the current no-context role in use */
    role(this: Bot, msg: D.Message) {
        let server = this.get_server(msg);
        if (!msg.guild || !server) {
            return;
        }
        if (server.no_context_role) {
            let rolename = msg.guild.roles.resolve(server.no_context_role)?.name;
            Logger.inspect(msg.guild.roles.cache);
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
                    index = lines.lastIndexOf(rolename!);
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
        const server = this.get_server(msg);
        if (!msg.guild || !server) {
            return; // Invalid
        }
        if (!server.hof_channel || !this.can_talk(server.hof_channel)) {
            // TODO reply something funny?
            return;
        }
        let self = this;
        let thischannel = msg.channel; // This must be an allowed channel already
        let [messageID] = parseArgs(msg, arg(argType.string));
        if (messageID) {
            let success = false;
            for (let [channel_id, channel] of msg.guild.channels.cache) {
                if (channel.isTextBased() && !server.disallowed_channels_listen.has(channel_id)) {
                    try {
                        let msg = await channel.messages.fetch(messageID as D.Snowflake);
                        if (msg.reactions.resolve(emojis.pushpin.toString())?.me) {
                            thischannel.send(rb_(self.text.pinAlreadyPinned, "That message is already pinned"));
                            return;
                        }
                        msg.react(emojis.pushpin.toString());
                        await self.pin(msg, server.hof_channel, emojis.exlamations);
                        success = true;
                        break;
                    } catch (err) {
                        // Logger.inspect(err);
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

    /** Reloads the text instanct of the bot. Useful if the text is changed while the bot is still running */
    async reload_text(this: Bot, msg: D.Message) {
        if (msg.author.id === this.ownerID) {
            let [res, err] = await this.loadText();
            if (res) {
                this.reply(msg, "Text reloaded successfully");
            } else {
                this.replyDM(msg, `Error loading text: \`\`\`${err}\`\`\``);
            }
        }
    },

    async servers(this: Bot, msg: D.Message) {
        const admin_powers = msg.author.id === this.ownerID;
        const self = this;

        if (!admin_powers && 
            (!msg.guild || !msg.guild.members.cache.get(msg.author.id)?.permissions.has(D.PermissionFlagsBits.Administrator))) {
            return;
        }
        
        let [serverid, subcmd, rest] = parseArgs(msg, arg(argType.string), arg(argType.string), arg(argType.rest));
        let guild: D.Guild | undefined | null;
        
        if (msg.channel.type !== D.ChannelType.DM) {
            guild = msg.guild;
            rest = `${subcmd} ${rest}`;
            subcmd = serverid;
        } else {
            guild = this.client.guilds.cache.get(serverid as D.Snowflake);
        }

        // [0] Argument it takes or undefined if it cannot be used
        // [1] Check function to verify value or undefined for no check
        // [2] Stringify function or undefined for default function
        type KeyData = [Arg | undefined, ((a: any) => boolean) | undefined, ((a: any) => string | undefined) | undefined];
        function kd(arg?: Arg, check?: ((a: any) => boolean), stringify?: ((a: any) => string | undefined)): KeyData {
            return [arg, check, stringify];
        }

        const args: {[T in keyof Server]: KeyData} = {
            id: kd(undefined, undefined, (id: D.Snowflake) => self.client.guilds.cache.get(id)?.name),
            _beta: kd(admin_powers ? arg(argType.boolean) : undefined),
            nickname: kd(arg(argType.rest, '', true), (s: string) => s.length <= 32, (s: string) => s), // null means clear
            allowed_channels_commands: kd(arg(argType.channel), undefined, (c: D.Snowflake) => guild?.channels.cache.get(c)?.name),
            disallowed_channels_listen: kd(arg(argType.channel), undefined, (c: D.Snowflake) => guild?.channels.cache.get(c)?.name),
            pin_amount: kd(arg(argType.number, 3, true), (n: number) => n > 0), // null means default
            hof_channel: kd(arg(argType.channel, undefined, true), undefined, (c: D.TextChannel) => c.name), // null means clear
            hof_emoji: kd(
                arg(argType.emoji, emojis.pushpin, true), 
                (e: Emoji) => e.id === null || (guild?.emojis.cache.has(e.id as D.Snowflake) ?? false),
                (e: Emoji) => `${e}`
            ), // null means default
            vague_channel: kd(arg(argType.channel, undefined, true), undefined, (c: D.TextChannel) => c.name), // null means clear
            vague_emoji: kd(
                arg(argType.emoji, emojis.pushpin, true), 
                (e: Emoji) => e.id === null || (guild?.emojis.cache.has(e.id as D.Snowflake) ?? false),
                (e: Emoji) => `${e}`
            ), // null means default
            word_wrong_channel: kd(arg(argType.channel, undefined, true), undefined, (c: D.TextChannel) => c.name), // null means clear
            word_wrong_emoji: kd(
                arg(argType.emoji, emojis.pushpin, true), 
                (e: Emoji) => e.id === null || (guild?.emojis.cache.has(e.id as D.Snowflake) ?? false),
                (e: Emoji) => `${e}`
            ), // null means default
            anything_pin_channel: kd(arg(argType.channel, undefined, true), undefined, (c: D.TextChannel) => c.name), // null means clear
            no_context_channel: kd(arg(argType.channel, undefined, true), undefined, (c: D.TextChannel) => c.name), // null means clear
            no_context_role: kd(arg(argType.role, undefined, true), undefined, (r: D.Role) => r.name), // null means clear
            titlecard_emoji: kd(
                arg(argType.emoji, undefined, true), 
                (e: Emoji | null) => e === null || e.id === null || (guild?.emojis.cache.has(e.id as D.Snowflake) ?? false),
                (e: Emoji | null) => `${e}`
            ),
            [util.inspect.custom]: kd(),
            allowed_commands: kd(),
            allowed_listen: kd(),
            as_jsonable: kd()
        };

        if (!guild && !subcmd) {
            show(this.servers);
            return;
        }

        if (!guild) {
            this.reply(msg, `Guild is unknown`);
            return;
        }

        function stringify_server(obj: Server) {
            let lines: string[] = [];
            for (let k in obj) {
                let key: keyof Server = k as keyof Server;
                let [valid, _, func] = args[key as keyof Server];
                if (!valid && !func) {
                    continue;
                }

                const val = obj[key];
                if (func && val) {
                    if (Array.isArray(val) || val instanceof Set) {
                        const text = Array.from(val).map((val) => {
                            const computed = func!(val);
                            if (computed) {
                                return computed === `${val}` ? `"${computed}"` : `"${computed}" // (${val})`
                            }
                            return `${val}`
                        }).join(',\n    ');
                        lines.push(`  ${key.toString()}: [\n    ${text}\n  ]`);
                    } else {
                        const computed = func(val);
                        let text: string = `${val}`;
                        if (computed) {
                            text = computed === text ? `"${computed}"` : `"${computed}" // (${val})`
                        }
                        lines.push(`  ${key.toString()}: ${text}`);
                    }
                } else if (!func) {
                    if (Array.isArray(val) || val instanceof Set) {
                        lines.push(`  ${key.toString()}: [\n    ${Array.from(val).map((val) => `${val}`).join(',\n    ')}\n  ]`);
                    } else {
                        lines.push(`  ${key.toString()}: ${val}`);
                    }
                } else {
                    lines.push(`  ${key.toString()}: null`);
                }
            }
            return `{\n${lines.join('\n')}\n}`;
        }

        function show(obj: Server | {[key: string]: Server}) {
            let lines: string[] = [];
            
            if (obj instanceof Server) {
                lines.push(stringify_server(obj));
            } else {
                for (let key in obj) {
                    lines.push(stringify_server(obj[key]));
                }
            }

            self.reply(msg, `\`\`\`js\n{\n${lines.join(',\n')}\n}\`\`\``);
            return;
        }

        if (!subcmd) {
            this.reply(msg, "Need a subcommand: `show` `info` `add` `remove` `set` `push` `pop`");
            return;
        }

        function primitive_filter<T extends keyof Server>(s: T): boolean {
            return args[s][0] !== undefined && 
            !Array.isArray(server[s]) && 
            !(server[s] instanceof Set) && 
            !(server[s] instanceof Function);
        }

        function set_filter<T extends keyof Server>(s: T): boolean {
            return args[s][0] !== undefined && 
            (server[s] instanceof Set) && 
            !(server[s] instanceof Function);
        }

        const server = this.servers[guild.id];

        switch (subcmd) {
            case 'show': {
                if (!server) {
                    this.reply(msg, 'To see all servers use `!servers` without any arguments');
                } else {
                    show(server);
                    return;
                }
            } break;
            case 'info': {
                if (server) {
                    let info: string[] = [];
                    info.push(`Info for ${guild.name}`);
                    for (let [channel_id, channel] of guild.channels.cache) {
                        if (!channel.isTextBased()) continue;

                        info.push(`<#${channel_id}>${channel.isThread() ? ` (Thread of <#${channel.parent?.id}>)` : ''}: Listen: \`${this.can_listen(channel) ? '`Y`' : '`N`'}\`. ` +
                                  `Commands: \`${this.can_command(channel) ? '`Y`' : '`N`'}\`. ` +
                                  `Speak: \`${this.can_talk(channel) ? '`Y`' : '`N`'}\``);
                    }
                    this.reply(msg, info.join('\n'));
                    return;
                } else {
                    this.reply(msg, `Guild ${guild.name} (${guild.id}) is not in the servers`);
                    return;
                }
            }
            case 'add': {
                if (!server) {
                    this.servers[guild.id] = new Server(this.client, guild.id, {_beta: !this.beta});
                    this.reply(msg, `Guild ${guild.name} (${guild.id}) added as ${this.beta ? 'not beta' : 'beta'}`);
                    show(this.servers[guild.id]);
                    this.save_servers();
                } else {
                    this.reply(msg, `Guild ${guild.name} (${guild.id}) is already in the servers`);
                    return;
                }
            } break;
            case 'remove': {
                if (server) {
                    delete this.servers[guild.id];
                    this.reply(msg, `Guild ${guild.name} (${guild.id}) deleted`);
                    this.save_servers();
                } else {
                    this.reply(msg, `Guild ${guild.name} (${guild.id}) is not in the servers`);
                    return;
                }
            } break;
            case 'set': // Fallthrough
            case 'push': // Fallthrough
            case 'pop': {
                const set = subcmd === 'set';
                const remove = subcmd === 'pop';

                if (!server) {
                    this.reply(msg, `Guild ${guild.id} has not been added yet, try \`!servers ${guild.id} add\``);
                    return;
                }

                const valid_keys = (Object.keys(server) as [keyof Server]).filter(set ? primitive_filter : set_filter);
                if (!rest) {
                    this.reply(msg, `\`${subcmd}\` needs a valid key: \`${valid_keys.join('`, `')}\``);
                    return;
                }

                let [param, restt] = parseArgsHelper(rest, guild, arg(argType.string), arg(argType.rest));
                let key: keyof Server | undefined | null = param as keyof Server & string | undefined | null;
                if (!key) {
                    this.reply(msg, `\`${subcmd}\` needs a valid key: \`${valid_keys.join('`, `')}\``);
                    return;
                }
                if (!valid_keys.includes(key)) {
                    this.reply(msg, `\`${param}\` is not a valid key to \`${subcmd}\`, use one of \`${valid_keys.join('`, `')}\``);
                    return;
                }

                const [argg, check, ...extra] = args[key];
                let [value] = parseArgsHelper(restt ?? '', guild, argg!);
                if (value === undefined || (check && !check(value))) {
                    this.reply(msg, `Value "${restt}" given to \`${subcmd}\` is invalid`);
                    return;
                }

                if (set) {
                    server[key] = value as never;
                    this.reply(msg, `Set \`${key}\` to \`${value}\``);
                } else {
                    const val = (value instanceof D.Role || value instanceof D.GuildMember || value instanceof D.TextChannel) ? value.id : value;
                    const arr = server[key] as Set<typeof val>;
                    if (remove && !arr.has(val)) {
                        this.reply(msg, `${val} is not in ${key}`);
                        return;
                    } else if (!remove && arr.has(val)) {
                        this.reply(msg, `${val} is in ${key}`);
                        return;
                    } else {
                        if (remove) {
                            arr.delete(val);
                            this.reply(msg, `Removed \`${val}\` from \`${key}\``);
                        } else {
                            arr.add(val);
                            this.reply(msg, `Added \`${val}\` to \`${key}\``);
                        }
                    }
                }
                show(server);
                this.save_servers();
            } break;
            default:
                this.reply(msg, `Unknown subcommand ${subcmd}`);
                break;
        }
    }

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
    colour: cmds.color,
    server: cmds.servers
}

/** Holds all commands that can only be ran if the bot is in beta mode */
export const beta_cmds: { [key: string]: CommandFunc} = {
    /** Gives debug info for the message that calls this command */
    debug(this: Bot, msg: D.Message) {
        Logger.inspect(util.inspect(msg, true, 3, true));
    },

    /** Sends a message to the no-context realm */
    decontext(this: Bot, msg: D.Message) {
        this.maybe_remove_context(msg);
    },

    /** Alias of servers, effectively only for beta bot */
    __servers: cmds.servers,

    /** Alias of die, effectively only kills beta bot */
    __die: cmds.die,
}
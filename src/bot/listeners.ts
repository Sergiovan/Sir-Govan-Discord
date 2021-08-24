import 'colors';
import * as D from 'discord.js';

import { Bot } from './bot';

import { Emoji, emojis } from '../defines';
import * as f from '../utils';
import { Logger } from '../utils';

let in_sigint = false; // Booo, npm, boooo
export type ListenerFunction = (this: Bot, ...args: any[]) => void;

const E = D.Constants.Events;
type ClientListener<K extends keyof D.ClientEvents> = (this: Bot, ...args: D.ClientEvents[K]) => void;

/** Holds all listeners that will never be changed or updated while the bot*/
export const fixed_listeners: { [key in keyof D.ClientEvents]?: ClientListener<key>} = {
    async [E.CLIENT_READY](this: Bot) {
        Logger.debug("Ready?");
        const self = this;

        this.owner = await this.client.users.fetch(this.ownerID);

        await this.update_users();
        this.setListeners(); // Listen only after users are done updating

        let rerandomize = () => {
            Logger.debug("Randomizing self");
            self.randomize_self();
            let milliseconds = 60 * 60 * 1000 + (Math.random() * (23 * 60 * 60 * 1000)); 
            self.randomize_timeout = setTimeout(rerandomize, milliseconds);
        };

        process.removeAllListeners('uncaughtException');
        process.removeAllListeners('SIGINT');

        process.on('uncaughtException', function(err: Error) {
            Logger.error(err.stack);
            Logger.debug("Bruh");
            self.die();
        });

        process.on('SIGINT', function() {
            if (!in_sigint) {
                in_sigint = true;
                
                Logger.debug("Buh bai");
                self.die();
            }
        });

        try {
            await this.load_servers();
        } catch (err) {
            Logger.error(`Could not load servers: ${err}`);
        }

        rerandomize();

        Logger.debug("Ready!");
    },

    [E.ERROR](this: Bot, err: Error) {
        Logger.error(err);

        this.client.destroy();
        this.clearListeners(); // Disable everything so things don't happ
        this.connect(); // TODO Will this megadie
    }
};

export const listeners: { [key in keyof D.ClientEvents]?: ClientListener<key>} = {
    async [E.MESSAGE_CREATE](this: Bot, msg: D.Message) {
        if (msg.channel.type === "DM") {
            // DMs, tread carefully
            const chn = msg.channel.partial ? await msg.channel.fetch() : msg.channel
            const channel_user = chn.recipient;
            let channel_name = `${channel_user.tag}`;
            const message_mine = msg.author.id === this.client.user!.id;
            if (!message_mine) {
                channel_name = 'me';
            }

            const author: string = message_mine ? 'me' : msg.author.tag;
            Logger.info(`${author.cyan} @ ${channel_name.cyan}: ${msg.cleanContent}`);

            if (message_mine) {
                return;
            }
            
            // TODO Huuuu
            if (this.parse(msg)) {
                return;
            }

            const sanitized = msg.cleanContent?.replace(/["'`]/g, '');
            
            if (sanitized) {
                const words = sanitized.split(' ');
                for (let word of words) {
                    this.checkAnswer(word, msg.author);
                }
            }
        } else {
            // Not DMs, tread as you wish
            const server = this.get_server(msg);
            if (!server) {
                return;
            }

            const can_command = this.can_command(msg);
            const can_listen = this.can_listen(msg);
            if (!can_command && !can_listen) {
                return;
            }

            const author: string = msg.author.id === this.client.user!.id ? 'me' : `${msg.author.tag}`;
            Logger.info(`${author.cyan} @ ${msg.channel.name.cyan}: ${msg.cleanContent}`);
            
            if (msg.author.id === this.client.user!.id) {
                return;
            }
            
            if (can_listen && !msg.author.bot) {
                if ((Math.random() * 100) < 1.0 && server.no_context_channel) {
                    this.maybe_remove_context(msg);
                } else {
                    this.tickUser(msg.author);
                }
            }

            if (can_command) {
                if (this.parse(msg)) {
                    return;
                }
            }
        }
    },

    async [E.MESSAGE_REACTION_ADD](this: Bot, reaction: D.MessageReaction | D.PartialMessageReaction, user: D.User | D.PartialUser) {
        try {
            reaction = reaction.partial ? await reaction.fetch() : reaction;
        } catch (error) {
            Logger.warning(`Message was removed before reaction could be processed: ${error}`);
            return;
        }

        const msg = await reaction.message.fetch(); // TODO Remove when bug is fixed in discord.js
        const server = this.get_server(msg);
        if (!msg.guild || !server) {
            return;
        }

        if (!this.can_listen(msg)) {
            return;
        }

        if (user.id === this.client.user!.id) {
            return;
        }

        const emoji = reaction.emoji; 
        
        reaction.users.fetch();

        switch (emoji.toString()) {
            // Pinning
            case server.hof_emoji.toString(): {
                const m = msg;
                this.maybe_pin(m, server.hof_emoji, server.hof_channel, 
                                server.hof_emoji.toString() === emojis.pushpin.toString() ? emojis.reddit_gold : server.hof_emoji);
                break;
            }
            case server.vague_emoji.toString(): {
                const m = msg;
                this.maybe_pin(m, server.vague_emoji, server.vague_channel, server.vague_emoji);
                break;
            }
            case server.word_wrong_emoji.toString(): {
                const m = msg;
                this.maybe_pin(m, server.word_wrong_emoji, server.word_wrong_channel, server.word_wrong_emoji);
                break;
            }
            // Retweeting
            case emojis.repeat_one.toString(): // fallthrough
            case emojis.repeat.toString(): {
                if (!this.can_talk(msg)) {
                    return;
                }
                const m = msg;
                const u = await msg.guild.members.fetch(user.id);
                if (!m || !u) {
                    return;
                }
                this.maybe_retweet(m, u, emoji.name === emojis.repeat.toString());
                break;
            }
            case emojis.devil.toString(): {
                const m = msg;
                const u = user.partial ? await user.fetch() : user;
                if (!u || !m) {
                    return;
                }
                this.maybe_steal(m, u);
                break;
            }
            case server.titlecard_emoji?.toString(): {
                const m = msg;
                const u = await msg.guild.members.fetch(user.id);
                if (!u || !m) {
                    return;
                }
                if (m.author.id !== u.id) {
                    return; // Only on your own messages
                }
                this.maybe_titlecard(m, u);
                break;
            }
            default: { // Chaos
                if (!server.anything_pin_channel) {
                    break;
                }
                const m = msg;
                const e = new Emoji({name: emoji.name ?? emoji.identifier, id: emoji.id, animated: emoji.animated ?? false});
                this.maybe_pin(m, e, server.anything_pin_channel, e);
                break;
            }
        }
    },

    [E.GUILD_MEMBER_ADD](this: Bot, member: D.GuildMember) {
        const server = this.servers[member.guild.id];
        
        if (!server) {
            return;
        }

        const usr = this.users[member.id]; 
        if (usr) {
            usr.update_member(member);
            usr.commit();
        } else {
            this.db.addUser(member.user, 1, member.user.bot ? 1 : 0, member.nickname).then((u) => this.add_user(u));
        }
    },

    [E.GUILD_MEMBER_REMOVE](this: Bot, member: D.GuildMember | D.PartialGuildMember) {
        const server = this.servers[member.guild.id];
        
        if (!server) {
            return;
        }

        const usr = this.users[member.id];
        if (usr) {
            usr.db_user.is_member = 0;
            usr.commit();
        }
    },

    [E.GUILD_MEMBER_UPDATE](this: Bot, old_member: D.GuildMember | D.PartialGuildMember, member: D.GuildMember) {
        const server = this.servers[member.guild.id];
        
        if (!server) {
            return;
        }

        const usr = this.users[member.id];
        if (usr) {
            usr.update_member(member);
            usr.commit();
        } else {
            this.db.addUser(member.user, 1, member.user.bot ? 1 : 0, member.nickname).then((u) => this.add_user(u));
        }
    },

    [E.USER_UPDATE](this: Bot, old_user: D.User | D.PartialUser, user: D.User) {
        const usr = this.users[user.id];
        if (usr) {
            usr.update_user(user);
            usr.commit();
        } else {
            this.db.addUser(user, 1, user.bot ? 1 : 0).then((u) => this.add_user(u));
        }
    }
};

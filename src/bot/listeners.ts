import 'colors';
import * as D from 'discord.js';

import { Bot } from './bot';

import { Emoji, emojis } from '../defines';
import { Logger } from '../utils';

let in_sigint = false; // Booo, npm, boooo
export type ListenerFunction = (this: Bot, ...args: any[]) => void;

const E = D.Events;
type ClientListener<K extends keyof D.ClientEvents> = (this: Bot, ...args: D.ClientEvents[K]) => void;

/** Holds all listeners that will never be changed or updated while the bot*/
export const fixed_listeners: { [key in keyof D.ClientEvents]?: ClientListener<key>} = {
    async [E.ClientReady](this: Bot) {
        Logger.debug("Ready?");
        const self = this;

        this.owner = await this.client.users.fetch(this.ownerID);

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

    [E.Error](this: Bot, err: Error) {
        Logger.error(err);

        this.client.destroy();
        this.clearListeners(); // Disable everything so things don't happ
        this.connect(); // TODO Will this megadie
    }
};

export const listeners: { [key in keyof D.ClientEvents]?: ClientListener<key>} = {
    async [E.MessageCreate](this: Bot, msg: D.Message) {
        if (msg.channel.type === D.ChannelType.DM) {
            // DMs, tread carefully
            const chn = msg.channel.partial ? await msg.channel.fetch() : msg.channel
            const channel_user = chn.recipient ?? {tag: 'Nobody'};
            let channel_name = `${channel_user.tag}`;
            const message_mine = msg.author.id === this.client.user!.id;
            if (!message_mine) {
                channel_name = 'me';
            }

            const content = msg.cleanContent.length ? ` ${msg.cleanContent}` : '';
            const attachments = msg.attachments.size ? ` [${msg.attachments.size} attachments]`.yellow : '';
            const stickers = msg.stickers.size ? ` [${msg.stickers.size} stickers]`.yellow : '';

            const author: string = message_mine ? 'me' : msg.author.tag;
            Logger.info(`${author.cyan} @ ${channel_name.cyan}:${content}${attachments}${stickers}`);

            if (message_mine) {
                return;
            }
            
            // TODO Huuuu
            if (this.parse(msg)) {
                return;
            }
            
        } else {
            // Not DMs, tread as you wish
            const server = this.get_server(msg);
            if (!server) {
                return;
            }

            const can_command = this.can_command(msg);
            const can_listen = this.can_listen(msg);
            const can_talk = this.can_talk(msg);
            if (!can_command && !can_listen) {
                return;
            }

            const content = msg.cleanContent.length ? ` ${msg.cleanContent}` : '';
            const attachments = msg.attachments.size ? ` [${msg.attachments.size} attachments]`.yellow : '';
            const stickers = msg.stickers.size ? ` [${msg.stickers.size} stickers]`.yellow : '';

            const author: string = msg.author.id === this.client.user!.id ? 'me' : `${msg.author.tag}`;
            Logger.info(`${author.cyan} @ ${msg.channel.name.cyan}:${content}${attachments}${stickers}`);
            
            if (msg.author.id === this.client.user!.id) {
                return;
            }
            
            if (can_listen && !msg.author.bot) {
                if ((Math.random() * 100) < 1.0 && server.no_context_channel) {
                    this.maybe_remove_context(msg);
                }
            }
            
            if (can_talk) {
                const words = content.trim().split(' ');
                if (words.length === 2 && words[1].toLowerCase().endsWith('ed')) {
                    if ((Math.random() * 10000) < 1.0 && msg.channelId === '140942235670675456') {
                        this.maybe_dark_souls(msg, emojis.fire_heart);
                    }
                }
            }

            if (can_command) {
                if (this.parse(msg)) {
                    return;
                }
            }
        }
    },

    async [E.MessageReactionAdd](this: Bot, reaction: D.MessageReaction | D.PartialMessageReaction, user: D.User | D.PartialUser) {
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
                this.maybe_titlecard(m, u);
                break;
            }
            case emojis.headstone.toString():
            case emojis.fire_heart.toString(): {
                if (!this.can_talk(msg)) {
                    return;
                }
                const m = msg;
                const u = await msg.guild.members.fetch(user.id);
            
                if (!u || !m) {
                    return;
                }
                const death = emoji.toString() === emojis.headstone.toString();
                this.maybe_dark_souls(m, u, death ? emojis.headstone : emojis.fire_heart, death ? 'YOU_DIED' : null);
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

    [E.GuildMemberAdd](this: Bot, member: D.GuildMember) {
        const server = this.servers[member.guild.id];
        
        if (!server) {
            return;
        }
    },

    [E.GuildMemberRemove](this: Bot, member: D.GuildMember | D.PartialGuildMember) {
        const server = this.servers[member.guild.id];
        
        if (!server) {
            return;
        }
    },

    [E.GuildMemberUpdate](this: Bot, old_member: D.GuildMember | D.PartialGuildMember, member: D.GuildMember) {
        const server = this.servers[member.guild.id];
        
        if (!server) {
            return;
        }
    },

    [E.UserUpdate](this: Bot, old_user: D.User | D.PartialUser, user: D.User) {

    }
};

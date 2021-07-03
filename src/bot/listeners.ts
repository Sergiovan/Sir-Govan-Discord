import 'colors';
import * as D from 'discord.js';

import { Bot } from './bot';

import { botparams, Emoji, emojis } from '../defines';
import * as f from '../utils';
import { assert } from 'console';

let in_sigint = false; // Booo, npm, boooo
export type ListenerFunction = (this: Bot, ...args: any[]) => void;

const E = D.Constants.Events;
type ClientListener<K extends keyof D.ClientEvents> = (this: Bot, ...args: D.ClientEvents[K]) => Awaited<void>;

/** Holds all listeners that will never be changed or updated while the bot*/
export const fixed_listeners: { [key in keyof D.ClientEvents]?: ClientListener<key>} = {
    async [E.CLIENT_READY](this: Bot) {
        const self = this;

        this.owner = await this.client.users.fetch(botparams.owner);
        await this.client.guilds.fetch();

        for (let [guild_id, guild] of this.client.guilds.cache) {
            const server = botparams.servers.ids[guild_id];
            if (!server || this.beta !== server.beta) {
                continue;
            }
            const new_nick = f.rb_(this.text.nickname, server.nickname || 'Sir Govan') + (this.beta ? ' (β)' : '');
            guild.me?.setNickname(new_nick);
            
        }

        await this.update_users();
        this.setListeners(); // Listen only after users are done updating

        process.removeAllListeners('uncaughtException');
        process.removeAllListeners('SIGINT');

        process.on('uncaughtException', function(err) {
            console.log(err);
            console.log("Bruh");
            self.die();
        });

        process.on('SIGINT', function() {
            if (!in_sigint) {
                in_sigint = true;
                
                console.log("Buh bai");
                self.die();
            }
        });

        console.log("Ready!");
    },

    [E.ERROR](this: Bot, err: Error) {
        console.error(err);

        this.client.destroy();
        this.clearListeners(); // Disable everything so things don't happ
        this.connect(); // TODO Will this megadie
    }
};

export const listeners: { [key in keyof D.ClientEvents]?: ClientListener<key>} = {
    [E.MESSAGE_CREATE](this: Bot, msg: D.Message) {
        if (msg.channel instanceof D.DMChannel) {
            // DMs, tread carefully
            const channel_user = msg.channel.recipient;
            let channel_name = `${channel_user.username}#${channel_user.discriminator}`;
            const message_mine = msg.author.id === this.client.user!.id;
            if (!message_mine) {
                channel_name = 'me';
            }

            // TODO Better logging
            const author: string = message_mine ? 'me' : msg.author.tag;
            console.log(`${author.cyan} @ ${channel_name.cyan}: ${msg.cleanContent}`);

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
            const server = botparams.servers.getServer(msg);
            if (!server) {
                return;
            }
            if (server.beta !== this.beta) {
                return;
            }
            if (!server.allowed(msg) && !server.allowedListen(msg)) {
                return;
            }

            const author: string = msg.author.id === this.client.user!.id ? 'me' : `${msg.author.username}#${msg.author.discriminator}`;
            console.log(`${author.cyan} @ ${msg.channel.name.cyan}: ${msg.cleanContent}`);
            
            if (msg.author.id === this.client.user!.id) {
                return;
            }
            
            if (server.allowedListen(msg) && !msg.author.bot) {
                if ((Math.random() * 100) < 1.0 && server.no_context_channel) {
                    this.maybe_remove_context(msg);
                } else {
                    this.tickUser(msg.author);
                }
            }

            if (server.allowed(msg)) {
                if (this.parse(msg)) {
                    return;
                }
            }
        }
    },

    async [E.MESSAGE_REACTION_ADD](this: Bot, reaction: D.MessageReaction, user: D.User | D.PartialUser) {
        const msg = reaction.message.partial ? await reaction.message.fetch() : reaction.message;
        const server = botparams.servers.getServer(msg);
        if (!msg.guild || !server) {
            return;
        }
        if (server.beta !== this.beta) {
            return;
        }
        if (!server.allowed(msg) && !server.allowedListen(msg)) {
            return;
        }
        if (user.id === this.client.user!.id) {
            // Actually...
            return;
        }

        const emoji = reaction.emoji;

        if (server.allowed(msg) || server.allowedListen(msg)) {
            switch (emoji.identifier) {
                // Retweeting
                case emojis.repeat_one.asReaction: // fallthrough
                case emojis.repeat.asReaction: {
                    const m = msg;
                    const u = await msg.guild.members.fetch(user.id);
                    if (!m || !u) {
                        return;
                    }
                    this.maybe_retweet(m, u, emoji.identifier === emojis.repeat.asReaction);
                    break;
                }
            }
        }
        if (server.allowedListen(msg)) {
            switch (emoji.identifier) {
                // Pinning
                case emojis.pushpin.asReaction: {
                    const m = msg;
                    this.maybe_pin(m, emojis.pushpin);
                    break;
                }
            }
        }
        if (server.allowed(msg)) {
            if (emoji.identifier === emojis.devil.asReaction) {
                const m = msg;
                const u = user.partial ? await user.fetch() : user;
                if (!u || !m) {
                    return;
                }
                this.maybe_steal(m, u);
            }
        }
    },

    [E.GUILD_MEMBER_ADD](this: Bot, member: D.GuildMember) {
        const server = botparams.servers.ids[member.guild.id];
        
        if (!server || server.beta !== this.beta) {
            return;
        }

        if (this.users[member.id]) {
            this.users[member.id].update_member(member);
            this.users[member.id].commit();
        } else {
            this.db.addUser(member.user, 1, member.user.bot ? 1 : 0, member.nickname).then((u) => this.add_user(u));
        }
    },

    [E.GUILD_MEMBER_REMOVE](this: Bot, member: D.GuildMember | D.PartialGuildMember) {
        const server = botparams.servers.ids[member.guild.id];
        
        if (!server || server.beta !== this.beta) {
            return;
        }

        if (this.users[member.id]) {
            this.users[member.id].db_user.is_member = 0;
            this.users[member.id].commit();
        }
    },

    [E.GUILD_MEMBER_UPDATE](this: Bot, old_member: D.GuildMember | D.PartialGuildMember, member: D.GuildMember) {
        console.log('Nickname updated :)');
        const server = botparams.servers.ids[member.guild.id];
        
        if (!server || server.beta !== this.beta) {
            return;
        }

        if (this.users[member.id]) {
            this.users[member.id].update_member(member);
            this.users[member.id].commit();
        } else {
            this.db.addUser(member.user, 1, member.user.bot ? 1 : 0, member.nickname).then((u) => this.add_user(u));
        }
    },

    [E.USER_UPDATE](this: Bot, old_user: D.User | D.PartialUser, user: D.User) {
        if (this.users[user.id]) {
            this.users[user.id].update_user(user);
            this.users[user.id].commit();
        } else {
            this.db.addUser(user, 1, user.bot ? 1 : 0).then((u) => this.add_user(u));
        }
    }
};

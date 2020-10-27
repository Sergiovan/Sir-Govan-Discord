import 'colors';
import Eris from 'eris';

import { Bot } from './bot';

import { botparams, Emoji, emojis } from '../defines';
import * as f from '../utils';

let in_sigint = false; // Booo, npm, boooo

// Listeners here run regardless of if the bot is ready or not
export const fixed_listeners: { [key: string]: CallableFunction } = {
    async ready(this: Bot) {
        const self = this;

        this.owner = this.client.users.get(botparams.owner);

        for (let [guild_id, guild] of this.client.guilds) {
            const server = botparams.servers.ids[guild_id];
            if (!server || this.beta !== server.beta) {
                continue;
            }
            const new_nick = f.rb_(this.text.nickname, server.nickname || 'Sir Govan') + (this.beta ? ' (β)' : '');
            guild.editNickname(new_nick);
            
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

    error(this: Bot, err: Error, id: number) {
        console.error(err, id);

        this.client.disconnect({reconnect: true});
        this.clearListeners(); // Disable everything so things don't happ
        this.client.connect();
    }
};

export const listeners: { [key: string]: CallableFunction } = {
    messageCreate(this: Bot, msg: Eris.Message) {
        if (!msg.guildID) {
            // DMs, tread carefully
            const channel_user = (msg.channel as Eris.PrivateChannel).recipient;
            let channel_name = `${channel_user.username}#${channel_user.discriminator}`;
            const message_mine = msg.author.id === this.client.user.id;
            if (!message_mine) {
                channel_name = 'me';
            }

            const author: string = message_mine ? 'me' : `${msg.author.username}#${msg.author.discriminator}`;
            console.log(`${author.cyan} @ ${channel_name.cyan}: ${msg.cleanContent}`);
            if (message_mine) {
                return;
            }
            
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

            const author: string = msg.author.id === this.client.user.id ? 'me' : `${msg.author.username}#${msg.author.discriminator}`;
            console.log(`${author.cyan} @ ${(msg.channel as Eris.TextChannel).name.cyan}: ${msg.cleanContent}`);
            
            if (msg.author.id === this.client.user.id) {
                return;
            }
            
            if (server.allowedListen(msg) && !msg.author.bot) {
                if ((Math.random() * 100) < 1.0 && server.no_context_channel) {
                    this.tryRemoveContext(msg, server);
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

    async messageReactionAdd(this: Bot, msg: Eris.Message, emoji: Emoji, user: string) {
        const server = botparams.servers.getServer(msg)
        if (!server) {
            return;
        }
        if (server.beta !== this.beta) {
            return;
        }
        if (!server.allowed(msg) && !server.allowedListen(msg)) {
            return;
        }
        if (user === this.client.user.id) {
            return;
        }

        if (server.allowedListen(msg)) {
            // Pinning
            if (emoji.name === emojis.pushpin.fullName) {
                const m = await msg.channel.getMessage(msg.id);
                this.maybe_pin(m, emoji);
            }
        }
        if (server.allowed(msg)) {
            if (emoji.name === emojis.devil.fullName) {
                const m = await msg.channel.getMessage(msg.id);
                const u = (msg.channel as Eris.TextChannel).guild.members.get(user)
                if (!u || !m) {
                    return;
                }
                this.maybe_steal(m, u.user);
            }
        }
    },

    guildMemberAdd(this: Bot, guild: Eris.Guild, member: Eris.Member) {
        const server = botparams.servers.ids[guild.id];
        
        if (!server || server.beta !== this.beta) {
            return;
        }

        if (this.users[member.id]) {
            this.users[member.id].update_member(member);
            this.users[member.id].commit();
        } else {
            this.db.addUser(member.user, 1, member.bot ? 1 : 0, member.nick).then((u) => this.add_user(u));
        }
    },

    guildMemberRemove(this: Bot, guild: Eris.Guild, member: Eris.Member | {id: string, user: Eris.User}) {
        const server = botparams.servers.ids[guild.id];
        
        if (!server || server.beta !== this.beta) {
            return;
        }

        if (this.users[member.id]) {
            this.users[member.id].db_user.is_member = 0;
            this.users[member.id].commit();
        }
    },

    guildMemberUpdate(this: Bot, guild: Eris.Guild, member: Eris.Member, oldMember: {roles: Array<string>, nick: string }) {
        const server = botparams.servers.ids[guild.id];
        
        if (!server || server.beta !== this.beta) {
            return;
        }

        if (this.users[member.id]) {
            this.users[member.id].update_member(member);
            this.users[member.id].commit();
        } else {
            this.db.addUser(member.user, 1, member.bot ? 1 : 0, member.nick).then((u) => this.add_user(u));
        }
    },

    userUpdate(this: Bot, user: Eris.User, oldUser: {username: string, discriminator: string, avatar: string}) {
        if (this.users[user.id]) {
            this.users[user.id].update_user(user);
            this.users[user.id].commit();
        } else {
            this.db.addUser(user, 1, user.bot ? 1 : 0).then((u) => this.add_user(u));
        }
    }
};

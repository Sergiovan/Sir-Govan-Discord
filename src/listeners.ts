"use strict";

import Eris, { PrivateChannel } from 'eris';

import { botparams, Emoji, emojis } from './defines';
import { Bot } from './bot';
import * as f from './utils';
import 'colors';

let in_sigint = false; // Booo, npm, boooo

export const listeners: { [key: string]: CallableFunction } = {
    ready(this: Bot) {
        let self = this;

        this._owner = this.client.users.get(botparams.owner);

        for (let [guild_id, guild] of this.client.guilds) {
            let server = botparams.servers.ids[guild_id];
            if (!server || this.beta !== server.beta) {
                continue;
            }
            let new_nick = f.rb_(this.text.nickname, server.nickname || 'Sir Govan') + (this.beta ? ' (β)' : '');
            guild.editNickname(new_nick);
            
        }

        this.update_users();

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

    messageCreate(this: Bot, msg: Eris.Message) {
        if (!msg.guildID) {
            // DMs, tread carefully
            let channel_user = (msg.channel as PrivateChannel).recipient;
            let channel_name = `${channel_user.username}#${channel_user.discriminator}`;
            let message_mine = msg.author.id === this.client.user.id;
            if (!message_mine) {
                channel_name = 'me';
            }

            let author: string = message_mine ? 'me' : `${msg.author.username}#${msg.author.discriminator}`;
            console.log(`${author.cyan} @ ${channel_name.cyan}: ${msg.cleanContent}`);
            if (message_mine) {
                return;
            }
            
            if (this.parse(msg)) {
                return;
            }

            let sanitized = msg.cleanContent?.replace(/["'`]/g, '');
            
            if (sanitized) {
                let words = sanitized.split(' ');
                for (let word of words) {
                    this.checkAnswer(word, msg.author);
                }
            }


        } else {
            // Not DMs, tread as you wish
            let server = botparams.servers.getServer(msg);
            if (!server) {
                return;
            }
            if (server.beta !== this.beta) {
                return;
            }
            if (!server.allowed(msg) && !server.allowedListen(msg)) {
                return;
            }
            let author: string = msg.author.id === this.client.user.id ? 'me' : `${msg.author.username}#${msg.author.discriminator}`;
            console.log(`${author.cyan} @ ${(msg.channel as Eris.TextChannel).name.cyan}: ${msg.cleanContent}`);
            if (msg.author.id === this.client.user.id) {
                return;
            }
            
            if (server.allowedListen(msg) && !msg.author.bot) {
                if ((Math.random() * 100) < 1.0 && server.no_context_channel) {
                    this.tryRemoveContext(msg, server);
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
        let server = botparams.servers.getServer(msg)
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
                let m = await msg.channel.getMessage(msg.id);
                this.maybe_pin(m, emoji);
            }
        }
        if (server.allowed(msg)) {
            if (emoji.name === emojis.devil.fullName) {
                let m = await msg.channel.getMessage(msg.id);
                let u = (msg.channel as Eris.TextChannel).guild.members.get(user)
                if (!u || !m) {
                    return;
                }
                this.maybe_steal(m, u.user);
            }
        }
    },

    error(this: Bot, err: Error, id: number) {
        console.error(err, id);

        this.client.disconnect({reconnect: true});
        this.client.connect();
    }
};

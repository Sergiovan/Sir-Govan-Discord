"use strict";

import Eris, { PrivateChannel } from 'eris';

import { botparams, Emoji, emojis } from './defines';
import { Bot } from './bot'
import * as f from './utils';
import 'colors';

export const listeners: { [key: string]: CallableFunction } = {
    ready(this: Bot) {
        let self = this;

        this.owner = this.client.users.get(botparams.owner);

        if (this.beta) {
            for (let [guild_id, guild] of this.client.guilds) {
                let server = botparams.servers.ids[guild_id];
                if (server.beta) {
                    if (server.nickname) {
                        guild.editNickname(server.nickname + ' (β)');
                    } else {
                        guild.editNickname(this.client.user.username + ' (β)');
                    }
                }
            }
        }

        process.removeAllListeners('uncaughtException');
        process.removeAllListeners('SIGINT');

        process.on('uncaughtException', function(err) {
            console.log(err);
            console.log("RIP me :(");
            self.die();
        });

        process.on('SIGINT', function() {
            console.log("Buh bai");
            self.die();
            process.exit(1);
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
                    let channel = server.no_context_channel;
                    if (msg.cleanContent?.length && msg.cleanContent.length <= 280 && !msg.attachments.length) {
                        this.client.createMessage(channel, msg.cleanContent);
                        if (server.no_context_role) {
                            for (let [_, member] of (msg.channel as Eris.TextChannel).guild.members) {
                                if (member.id === msg.author.id) {
                                    member.addRole(server.no_context_role);
                                } else if (member.roles.includes(server.no_context_role)) {
                                    member.removeRole(server.no_context_role);
                                }
                            }
                            f.randFromFile('nocontext.txt', 'No context', function(name) {
                                (msg.channel as Eris.TextChannel).guild.roles.get(server!.no_context_role)?.edit({name: name});
                            });

                            if (Math.random() * 4 < 1.0) {
                                this.postClue(server.allowed(msg) ? msg.channel.id : server.allowed_channels[0]);
                            }
                        }
                    }
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

        let self = this;
        if (server.allowedListen(msg)) {
            // Pinning
            if (emoji.name === emojis.pushpin.fullName) {
                let m = await msg.channel.getMessage(msg.id);
                pin(m, emoji);
            }
        }
        if (server.allowed(msg)) {
            if (emoji.name === emojis.devil.fullName) {
                let m = await msg.channel.getMessage(msg.id);
                let u = (msg.channel as Eris.TextChannel).guild.members.get(user)
                if (!u || !m) {
                    return;
                }
                steal(m, u.user);
            }
        }

        async function pin(msg: Eris.Message, emoji: Emoji) {
            let findname = emoji.id ? `${emoji.name}:${emoji.id}` : emoji.name;
            if (msg.author.bot) {
                return;
            }
            if (msg.reactions[emojis.pushpin.fullName] && msg.reactions[emojis.pushpin.fullName].me) {
                return;
            }
            let reactionaries = await msg.getReaction(findname, 4);
            if(reactionaries.filter((user) => user.id !== msg.author.id).length >= 3){
                //We pin that shit!
                msg.addReaction(emojis.pushpin.fullName);
                self.pin(msg);
            }
        }

        async function steal(msg: Eris.Message, user: Eris.User) {
            if (!msg.reactions[emojis.devil.fullName].me) {
                return;
            }

            let content = msg.content!;
            await msg.removeReaction(emojis.devil.fullName);
            await msg.edit(`Stolen by ${user.username}`);

            (await user.getDMChannel()).createMessage(content);
            setTimeout(() => msg.delete(), 1000 * 5 * 60);
        }

    },

    error(this: Bot, err: Error, id: number) {
        console.error(err, id);

        this.client.disconnect({reconnect: true});
        this.client.connect();
    }
};

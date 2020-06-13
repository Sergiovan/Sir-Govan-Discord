"use strict";

import Eris from 'eris';

import { botparams, Emoji, emojis } from './defines';
import { Bot } from './bot'
import * as f from './utils';
import 'colors';

export const listeners: { [key: string]: CallableFunction } = {
    ready(this: Bot) {
        let self = this;

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
        console.log(`${msg.author.username.cyan} @ ${(msg.channel as Eris.TextChannel).name.cyan}: ${msg.cleanContent}`);
        if (msg.author.id === this.client.user.id) {
            return;
        }
        
        if (server.allowedListen(msg) && !msg.author.bot) {
            if ((Math.random() * 100) < 1.0 && server.no_context_channel) {
                let channel = server.no_context_channel;
                if (msg.cleanContent?.length && msg.cleanContent.length <= 280 && !msg.attachments.length) {
                    this.client.createMessage(channel, msg.cleanContent);
                    if (server.no_context_role){
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
                    }
                }
            }   
        }
        if (server.allowed(msg)) {
            if (this.parse(msg)) {
                return;
            }
        }
    },

    messageReactionAdd(this: Bot, msg: Eris.Message, emoji: Emoji, user: string) {
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

        let self = this;
        if (server.allowedListen(msg)) {
            // Pinning
            if (emoji.name === emojis.pushpin.fullName) {
                msg.channel.getMessage(msg.id)
                    .then((rmsg) => pin(rmsg, emoji))
                    .catch((err) => {throw err;});
            }
        }

        function pin(msg: Eris.Message, emoji: Emoji){
            let findname = emoji.id ? `${emoji.name}:${emoji.id}` : emoji.name;
            if (msg.author.bot) {
                return;
            }
            if (msg.reactions[emojis.pushpin.fullName] && msg.reactions[emojis.pushpin.fullName].me) {
                return;
            }
            msg.getReaction(findname, 4)
                .then(function(reactionaries){
                    if(reactionaries.filter((user) => user.id !== msg.author.id).length >= 3){
                        //We pin that shit!
                        msg.addReaction(emojis.pushpin.fullName);
                        self.pin(msg);
                    }
                })
                .catch((err) => { throw err; });
        }
    }
};

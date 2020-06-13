"use strict";

import Eris from 'eris';
import * as fs from 'fs';
import * as path from 'path';

let c = require('./defines.js');
let f = require('./utils.js');

const parseArgs = f.parseArgs;
const arg = f.arg;

module.exports = {
    die(msg: Eris.Message) {
        if (msg.author.id === c.botparams.owner) {
            this.die();
        }
    },

    roll(msg: Eris.Message){
        let [err, num] = parseArgs(msg, arg(0, '20'));
        if (!err) {
            try {
                num = BigInt(num);
            } catch (e) {
                num = 20n;
            }
            this.createMessage(msg.channel.id, f.randomBigInt(num, 1n));
        }
    },

    color(msg: Eris.Message){
        let [err, color] = parseArgs(msg, arg(c.argTypes.string, Math.floor(Math.random() * 0x1000000)));
        if(!err){
            let number = Number.parseInt('0x' + (''+color).replace(/^(#|0x)/, ''));
            if(!Number.isNaN(number) && Number.isFinite(number) && number >= 0 && number < 0x1000000){
                let member = msg.member;
                let guild = msg.channel.guild;
                let roles = guild.roles;
                let user_roles = roles.filter((role) => member.roles.includes(role.id));
                user_roles.sort((a, b) => b.position - a.position);
                user_roles[0].edit({color: number});
            }else if(Number.isNaN(number)){
                this.createMessage(msg.channel.id, "That's not a valid color hex. Give me a valid hex, like #C0FFEE or #A156F2");
            }else if(!Number.isFinite(number)){
                this.createMessage(msg.channel.id, "That color would blow your mind. Give me a valid hex, like #0084CA or #F93822");
            }else if(number < 0){
                this.createMessage(msg.channel.id, "A negative hex? Now I know you're just fucking with me");
            }else{
                this.createMessage(msg.channel.id, "I'm unsure your monitor can even render that. Your hex is too high. " +
                    "Give me a valid hex, like #00AE8F, or #F2F0A1");
            }
        }else{
            this.createMessage(msg.channel.id, "Incredibly, something went wrong. I will now tell my master about it");
            console.log('Something went very wrong when changing colors :<'.red);
        }
    },

    role(msg: Eris.Message){
        let self = this;
        let server = c.botparams.servers.getServer(msg);
        if(!server) {
            return;
        }
        if(server.no_context_role){
            let rolename = msg.channel.guild.roles.get(server.no_context_role).name;
            fs.readFile(path.join('data', 'nocontext.txt'), "utf8", function(err, data) {
                let index = -1;
                let total = 0;
                if(err) {
                    console.log(`Error detected: ${err}`);
                } else {
                    let lines = data.trim().split('\n');
                    index = lines.indexOf(rolename);
                    total = lines.length;
                }
                rolename += index === -1 ? "\nNote: This role does not exist anymore. It's a shiny!" : "";
                index = index === -1 ? 'NaN' : `${index+1}/${total}`;
                self.createMessage(msg.channel.id, `${index}: ${rolename}`);
            });
        } else {
            this.createMessage(msg.channel.id, "This server does not have roles to collect. Sorry!");
        }
    },

    pin(msg){ 
        let self = this;
        let thischannel = msg.channel;
        let [err, messageID] = parseArgs(msg, arg(c.argTypes.string));
        if(!err){
            if(messageID){
                let server = c.botparams.servers.getServer(msg);
                if(server) {
                    for(let elem of msg.channel.guild.channels) {
                        let [_, channel] = elem;
                        if(server.allowed_channels_listen.includes(channel.id)) {
                            channel.getMessage(messageID)
                                .then(function(msg){
                                    if(msg.reactions[c.emojis.pushpin.fullName] && msg.reactions[c.emojis.pushpin.fullName].me) {
                                        self.createMessage(thischannel.id, "I already pinned that message >:(");
                                        return;
                                    }
                                    msg.addReaction(c.emojis.pushpin.fullName);
                                    self.pin(msg, true);
                                })
                                .catch(function(err){
                                    console.log(`Message not in ${channel.name}: ${err}`);
                                });
                        }
                    }
                }
            }else{
                this.createMessage(msg.channel.id, "You're gonna have to give me a message ID, pal");
            }
        }else{
            this.createMessage(msg.channel.id, "Hm. Something went wrong there");
        }
    },

    // pinall(msg){
    //     if(msg.author.id !== c.botparams.owner){
    //         return;
    //     }
    //     let [err, channel] = parseArgs(msg, arg(c.argTypes.channel));
    //     if(!err){
    //         if(channel) {
    //             channel.getPins()
    //                 .then((msgs) => {
    //                     msgs.reverse();
    //                     for(let msg of msgs) {
    //                         let emoji = c.emojis.pushpin.fullName;
    //                         if(msg.reactions[emoji] && msg.reactions[emoji].me) {
    //                             continue;
    //                         }
    //                         msg.addReaction(emoji);
    //                         this.pin(msg);
    //                     }
    //                 })
    //                 .catch((err) => console.log(`Something went wrong: ${err}`));
    //         } else {
    //             this.createMessage(msg.channel.id, "I need a proper channel name");
    //         }
    //     }else{
    //         this.createMessage(msg.channel.id, "Something went terribly wrong");
    //     }
    // },

    // fixpin(msg){
    //     let util = require('util');
    //     if(msg.author.id !== c.botparams.owner){
    //         return;
    //     }
    //     let [err, messageID] = parseArgs(msg, arg(c.argTypes.string));
    //     if (err) {
    //         this.createMessage(msg.channel.id, "Something went terribly wrong");
    //         return;
    //     }
    //     let pc = c.botparams.servers.getServer(msg).pin_channel;
    //     let ch = msg.channel.guild.channels.find(x => x.id === pc);
    //     let self = this;
    //     let bot_id = self.user.id;

    //     ch.getMessage(messageID).then(function(msg) {
    //         if (!msg.author || msg.author.id !== bot_id || 
    //             !msg.embeds || msg.embeds.length !== 1 || msg.embeds[0].type !== 'rich' || !msg.embeds[0].footer) {
    //                 console.log(`Invalid message: ${util.inspect(err, true, 4, true)}`);
    //                 return;
    //             }
    //         // console.log(msg.embeds[0].footer);
    //         let parts = msg.embeds[0].footer.text.match(/([0-9]+)(?: - ([0-9]+))?/);
    //         console.log(util.inspect(parts));
    //         let guild_id = msg.channel.guild.id;
    //         let channel_id = parts[2] || guild_id;
    //         let message_id = parts[1];
    //         let url = `https://canary.discordapp.com/channels/${guild_id}/${channel_id}/${message_id}`;
    //         let desc = `[Click to teleport](${url})`;
    //         if (!msg.embeds[0].description || !msg.embeds[0].description.length || msg.embeds[0].description === desc) {
    //             msg.embeds[0].description = desc;
    //         } else {
    //             msg.embeds[0].fields = [{
    //                 "name": "\u200b",
    //                 "value": `[Click to teleport](${url})`
    //             }];
    //         }
    //         ch.editMessage(msg.id, {embed: msg.embeds[0]});
    //     }).catch(function(err) {
    //         console.log(`Message not in ${ch.name}: ${err}`);
    //     });
    // },

    // __fixpin(msg) {
    //     let util = require('util');
    //     if(msg.author.id !== c.botparams.owner){
    //         return;
    //     }
    //     let pinid = '422800566713057282';
    //     let msgid = '251343460454498304';

    //     let pinchid  = '422796631235362841';
    //     let msgchid  = '140942235670675456'; 

    //     let guild = this.guilds.find(x => x.id === '140942235670675456');

    //     let pinchannel = guild.channels.find(x => x.id === pinchid);
    //     let messagechannel = guild.channels.find(x => x.id === msgchid);

    //     pinchannel.getMessage(pinid).then(function(msg){
    //         messagechannel.getMessage(msgid).then(function(rmsg){
    //             msg.embeds[0].image = {
    //                 url: rmsg.attachments[0].url,
    //                 height: rmsg.attachments[0].height,
    //                 width: rmsg.attachments[0].width
    //             };

    //             pinchannel.editMessage(msg.id, {embed: msg.embeds[0]});
    //         });
    //     });
    // }
};

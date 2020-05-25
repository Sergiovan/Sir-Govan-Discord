"use strict"; // Oh boy

let fs = require('fs');
let path = require('path');
let request = require('request');
let colors = require('colors');
let Eris = require('eris');
let util = require('util');

let c = require('./defines.js');
let s = require('./secrets.js');
let cmds = require('./commands.js');
let l = require('./listeners.js');

let bot = new Eris(s.discord.token);

if(!bot.commands){
    bot.commands = [];
}

if(bot.beta === undefined){
    bot.beta = process.argv.includes('--beta');
}

bot.parse = function(msg){
    let message = msg.content;
    for(let [commandName, command] of this.commands){
        if(message.split(' ')[0] === commandName){
            command.call(this, msg);
            return true;
        }
    }
    return false;
};

bot.addCommand = function(name, command){
    this.commands.push([name, command]);
};

bot.die = function(){
    for(let [guild_id, guild] of bot.guilds){
        let server = c.botparams.servers[guild_id];
        if(!server || bot.beta !== server.beta) {
            continue;
        }
        if(server.nickname){
            guild.editNickname(server.nickname);
        }else{
            guild.editNickname('');
        }
    }
    bot.disconnect({reconnect: false});
};

bot.pin = function(msg, forced = false){
    let server = c.botparams.servers.getServer(msg);
    let pinchannel = server.pin_channel;
    if(!pinchannel) {
        console.log("Can't pin this >:(");
        return false;
    } else {
        let icon = forced ? 'https://emojipedia-us.s3.amazonaws.com/thumbs/120/twitter/131/double-exclamation-mark_203c.png' : 'https://cdn.discordapp.com/emojis/263774481233870848.png';
        let r = Math.floor(Math.random() * 0x10) * 0x10;
        let g = Math.floor(Math.random() * 0x10) * 0x10;
        let b = Math.floor(Math.random() * 0x10) * 0x10;
        let embed = {
            color: r << 16 | g << 8 | b,
            author: {
                name: `${msg.author.username}`,
                icon_url: msg.author.dynamicAvatarURL("png", 128)
            },
            // thumbnail: {
            //     url: msg.author.dynamicAvatarURL("png", 128)
            // },
            description: `${msg.content}`,
            timestamp: new Date(msg.timestamp).toISOString(),
            footer: {
                text: `${msg.id} - ${msg.channel.id}`,
                icon_url: icon
            }
        };
        let guild_id = server.id;
        let channel_id = msg.channel.id;
        let message_id = msg.id;
        let url = `https://canary.discordapp.com/channels/${guild_id}/${channel_id}/${message_id}`;
        let desc = `[Click to teleport](${url})`;
        if(!embed.description) {
            embed.description = desc;
        } else {
            embed.fields = [{
                "name": "\u200b",
                "value": desc
            }];
        }
        if(msg.attachments && msg.attachments.length){
            let attachment = msg.attachments[0];
            let embedtype = /\.(webm|mp4)$/g.test(attachment.filename) ? 'video' : 'image';
            console.log(embedtype, attachment.filename);
            embed[embedtype] = {
                url: attachment.url,
                height: attachment.height,
                width: attachment.width
            };
        } else if (msg.embeds && msg.embeds.length) {
            let nembed = msg.embeds[0];
            if(nembed.video) { embed.video = nembed.video; }
            if(nembed.thumbnail) { embed.thumbnail = nembed.thumbnail; }
            if(nembed.image) { embed.image = nembed.image; }
        }
        this.createMessage(pinchannel, {embed: embed});
        return true;
    }
}

for(let command_name in cmds){
    if(!cmds.hasOwnProperty(command_name)){
        continue;
    }
    bot.addCommand(`!${command_name}`, cmds[command_name]);
}

bot.addCommand('!colour', cmds.color)

if(bot.beta){
    bot.addCommand('!debug', function(msg){
        console.log(util.inspect(msg, true, 5, true));
    });
    bot.addCommand('!__die', cmds['die']);
}

for(let event in l){
    if(!l.hasOwnProperty(event)){
        continue;
    }
    bot.removeAllListeners(event);
    bot.on(event, l[event]);
}

bot.connect();

/*
attachments:
   [ { url: 'https://cdn.discordapp.com/attachments/120581475912384513/403281342231740416/unknown.png',
       proxy_url: 'https://media.discordapp.net/attachments/120581475912384513/403281342231740416/unknown.png',
       filename: 'unknown.png',
       width: 1921,
       height: 1080,
       id: '403281342231740416',
       size: 2234944 } ],
*/
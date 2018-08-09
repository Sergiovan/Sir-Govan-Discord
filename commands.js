"use strict";

let c = require('./defines.js');
let f = require('./utils.js');

const parseArgs = f.parseArgs;
const arg = f.arg;

module.exports = {
    die(msg){
        if(msg.author.id === c.botparams.owner){
            this.die();
        }
    },

    roll(msg){
        let [err, num] = parseArgs(msg, arg(1, 20));
        if(!err){
            this.createMessage(msg.channel.id, Math.floor(Math.random()*num) + 1);
        }
    },

    color(msg){
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

    pinall(msg){
        if(msg.author.id !== c.botparams.owner){
            return;
        }
        let [err, channel] = parseArgs(msg, arg(c.argTypes.channel));
        if(!err){
            if(channel) {
                channel.getPins()
                    .then((msgs) => {
                        msgs.reverse();
                        for(let msg of msgs) {
                            let emoji = c.emojis.pushpin.fullName;
                            if(msg.reactions[emoji] && msg.reactions[emoji].me) {
                                continue;
                            }
                            msg.addReaction(emoji);
                            this.pin(msg);
                        }
                    })
                    .catch((err) => console.log(`Something went wrong: ${err}`));
            } else {
                this.createMessage(msg.channel.id, "I need a proper channel name");
            }
        }else{
            this.createMessage(msg.channel.id, "Something went terribly wrong")
        }
    }
};
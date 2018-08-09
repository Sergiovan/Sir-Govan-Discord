"use strict";

let c = require('./defines.js');
let f = require('./utils.js');
let cmds = require('./commands.js');

module.exports = {
    ready(){
        let self = this;

        if(this.beta){
            for(let [guild_id, guild] of this.guilds){
                if(c.botparams.servers[guild_id].beta){
                    if(c.botparams.servers[guild_id].nickname){
                        guild.editNickname(c.botparams.servers[guild_id].nickname + ' (β)');
                    }else{
                        guild.editNickname(this.user.username + ' (β)');
                    }
                }
            }
        }

        process.on('uncaughtException', function(err){
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

    messageCreate(msg){
        let server = c.botparams.servers.getServer(msg);
        if(!server) {
            return;
        }
        if(server.beta !== this.beta) {
            return;
        }
        if(!server.allowed(msg) && !server.allowedListen(msg)) {
            return;
        }
        console.log(`${msg.author.username.cyan} @ ${msg.channel.name.cyan}: ${msg.cleanContent}`);
        if(msg.author.id === this.user.id){
            return;
        }
        
        if(server.allowedListen(msg) && !msg.author.bot){
            if((Math.random() * 100) < 1.0 && server.no_context_channel) {
                let channel = server.no_context_channel;
                if(msg.cleanContent.length && msg.cleanContent.length <= 280 && !msg.attachments.length) {
                    this.createMessage(channel, msg.cleanContent);
                    if(server.no_context_role){
                        for(let [_, member] of msg.channel.guild.members) {
                            if(member.id === msg.author.id) {
                                member.addRole(server.no_context_role);
                            } else if(member.roles.includes(server.no_context_role)) {
                                member.removeRole(server.no_context_role);
                            }
                        }
                        f.randFromFile('nocontext.txt', 'No context', function(name) {
                            msg.channel.guild.roles.get(server.no_context_role).edit({name: name});
                        });
                    }
                }
            }   
        }
        if(server.allowed(msg)){
            if(this.parse(msg)){
                return;
            }
        }
    },

    messageReactionAdd(msg, emoji, user){
        let server = c.botparams.servers.getServer(msg)
        if(!server) {
            return;
        }
        if(server.beta !== this.beta) {
            return;
        }
        if(!server.allowed(msg) && !server.allowedListen(msg)){
            return;
        }

        let self = this;
        if(server.allowedListen(msg)){
            // Pinning
            if(emoji.name === c.emojis.pushpin.fullName){
                msg.channel.getMessage(msg.id)
                    .then((rmsg) => pin(rmsg, emoji))
                    .catch((err) => {throw err;});
            }
        }

        /**
         *
         * @param msg {Message}
         * @returns {Promise.<void>}
         */
        function pin(msg, emoji){
            let findname = emoji.id ? `${emoji.name}:${emoji.id}` : emoji.name;
	    if(msg.author.bot){
		return;
	    }
            if(msg.reactions[c.emojis.pushpin.fullName] && msg.reactions[c.emojis.pushpin.fullName].me) {
                return;
            }
            msg.getReaction(findname, 4)
                .then(function(reactionaries){
                    if(reactionaries.filter((user) => user.id !== msg.author.id).length >= 3){
                        //We pin that shit!
                        msg.addReaction(c.emojis.pushpin.fullName);
                        self.pin(msg);
                    }
                })
                .catch((err) => {throw err;});
        }
    }
};

"use strict";

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const {botparams, emojis, argTypes, ...rest} = require('./defines.js');

module.exports = {

    /**
     *
     * @param type {Number}
     * @param def {*}
     * @returns {[Number,*]}
     */
    arg(type, def = null){
        return [type, def];
    },

    /**
     *
     * @param {Message} msg
     * @param args
     * @returns {boolean[]}
     */
    parseArgs(msg, ...args){
        let ret = [false];
        let word = 1;
        let words = msg.content.replace(/ +/g, ' ').split(' ');
        let specialHolder;
        for(let [argType, argDef] of args){
            let inspected = words[word];
            switch(argType){
                case argTypes.string:
                    ret.push(inspected || argDef);
                    break;
                case argTypes.number:
                    if(Number.isFinite(+inspected) && !Number.isNaN(+inspected)){
                        ret.push(+inspected);
                    }else{
                        if(argDef){
                            ret.push(argDef);
                            word--;
                        }else{
                            return [true];
                        }
                    }
                    break;
                case argTypes.user:
                    specialHolder = /<@!?([0-9]+?)>/.exec(inspected);
                    if(specialHolder && specialHolder[1]){
                        if(msg.channel.guild.members.get(specialHolder[1])){
                            ret.push(msg.channel.guild.members.get(specialHolder[1]));
                            break;
                        }
                    }
                    return [true];
                case argTypes.channel:
                    specialHolder = /<#([0-9]+?)>/.exec(inspected);
                    if(specialHolder && specialHolder[1]){
                        if(msg.channel.guild.channels.get(specialHolder[1])){
                            ret.push(msg.channel.guild.channels.get(specialHolder[1]));
                            break;
                        }
                    }
                    return [true];
                case argTypes.role:
                    specialHolder = /<@\&([0-9]+?)>/.exec(inspected);
                    if(specialHolder && specialHolder[1]){
                        if(msg.channel.guild.roles.get(specialHolder[1])){
                            ret.push(msg.channel.guild.roles.get(specialHolder[1]));
                            break;
                        }
                    }
                    return [true];
                case argTypes.rest:
                    ret.push(words.slice(word).join(' '));
                    break;
                default:
                    throw new Error('Undefined argument');
            }
            word++;
        }
        return ret;
    },

    randFromFile(filename, def, cb) {
        fs.readFile(path.join('data', filename), "utf8", function(err, data) {
            if(err) {
                console.log(`Error detected: ${err}`);
                cb(def);
            } else {
                let lines = data.trim().split('\n');
                cb(lines[Math.floor(Math.random() * lines.length)]);
            }
        });
    },

    randomBigInt(max = 20n, min = 0n) {
        let r;
        const range = 1n + max - min;
        const bytes = BigInt(Math.ceil(range.toString(2).length / 8));
        const bits = bytes * 8n;
        const buckets = (2n ** bits) / range;
        const limit = buckets * range;

        do {
            r = BigInt('0x' + crypto.randomBytes(Number(bytes)).toString('hex'));
        } while (r >= limit);

        return min + (r / buckets);
    }
};

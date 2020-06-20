"use strict"; // Oh boy

import Eris from 'eris';

import { botparams, emojis } from './defines';
import { randomCode, randomEnum } from './utils';
import { CommandFunc, cmds } from './commands';
import { ClueType, ClueGenerator, mysteryGenerator, clueHelp } from './secrets';
import { createHash } from 'crypto';

type Command = [string, (msg: Eris.Message) => void];

export class Bot {
    client: Eris.Client;

    commands: Command[] = [];
    beta: boolean;

    clue: string = '';
    clue_type: ClueType = ClueType.LetterPosition;
    clue_gen?: ClueGenerator;
    last_clue: Date = new Date(0);
    puzzle_id: string = '';

    owner?: Eris.User;

    constructor(token: string, beta: boolean) {
        this.client = new Eris.Client(token);

        this.beta = beta;

        let self = this;

        setTimeout(function check_connect() {
            if (self.owner) {
                self.startClues();
            } else {
                setTimeout(check_connect, 1000);
            }
        }, 1000);
    }

    parse(msg: Eris.Message) {
        let message = msg.content;
        for(let [commandName, command] of this.commands){
            if(message.split(' ')[0] === commandName){
                command.call(this, msg);
                return true;
            }
        }
        return false;
    }

    addCommand(name: string, command: CommandFunc) {
        this.commands.push([name, command]);
    }

    setEventListener(name: string, handler: CallableFunction) {
        this.client.removeAllListeners(name);
        this.addEventHandler(name, handler);
    }

    addEventHandler(name: string, handler: CallableFunction) {
        this.client.on(name, handler.bind(this));
    }

    startClues() {
        this.clue = randomCode();
        this.clue_type = randomEnum(ClueType);
        this.startGenerator();

        let hasher = createHash('md5');
        hasher.update(this.clue);
        this.puzzle_id = hasher.digest('hex').substr(0, 8);

        this.owner!.getDMChannel().then((ch) => ch.createMessage(`Puzzle started: ${this.clue}. ID: \`${this.puzzle_id}\``));
        console.log(`New clue game started: Clue is ${this.clue}. ID is ${this.puzzle_id}`);
    }

    startGenerator() {
        this.clue_gen = mysteryGenerator(this.clue, this.clue_type);
    }

    canGetClue() {
        console.log(new Date().getTime() - (1000 * 60 * 60),  this.last_clue.getTime(), this.clue_gen);
        return (new Date().getTime() - (1000 * 60 * 60) > this.last_clue.getTime()) && this.clue_gen;
    }

    getClue() {
        if (!this.canGetClue()) {
            return null;
        }

        let res = this.clue_gen!.next();
        if (res.done) {
            this.startGenerator();
            let ret = this.clue_gen!.next().value || null;
            if (ret) {
                this.last_clue = new Date();
            }
            return ret;
        }
        this.last_clue = new Date();
        return res.value;
    }

    async postClue(channel: string) {
        console.log('Posting clue');
        if (!this.canGetClue()) {
            return null;
        }
        let msg = await this.client.createMessage(channel, 'Generating clue...');
        await msg.addReaction(emojis.devil.fullName);
        let clue = this.getClue();
        await msg.edit(`\`${clue}\``);
    }

    async checkAnswer(answer: string, user: Eris.User) {
        if (!this.clue?.length || !this.owner) {
            return;
        }
        

        if (answer === this.clue) {
            this.clue = '';
            this.clue_gen = undefined;

            let dm = await user.getDMChannel();
            dm.createMessage('You got it!');

            (await this.owner.getDMChannel()).createMessage(`${user.username} (${user.id}) got it!`);

            setTimeout(this.startClues.bind(this), 1000 * 60 * 60 * 24);
        }
    }

    puzzleHelp(): string {
        if (!this.clue) {
            return 'Nothing going on at the moment';
        } else {
            return `Complete the passphrase and tell it to me for prizes. The clue is: ||${clueHelp(this.clue_type)}||\nPuzzle ID is \`${this.puzzle_id}\``;
        }
    }

    pin(msg: Eris.Message, forced: boolean = false) {
        let server = botparams.servers.getServer(msg);
        let pinchannel = server?.pin_channel;
        if (!pinchannel) {
            console.log("Can't pin this >:(");
            return false;
        } else {
            let icon = forced ? 'https://emojipedia-us.s3.amazonaws.com/thumbs/120/twitter/131/double-exclamation-mark_203c.png' : 'https://cdn.discordapp.com/emojis/263774481233870848.png';
            let r = Math.floor(Math.random() * 0x10) * 0x10;
            let g = Math.floor(Math.random() * 0x10) * 0x10;
            let b = Math.floor(Math.random() * 0x10) * 0x10;
            let embed: Eris.Embed = {
                type: 'rich',
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
            let guild_id = server!.id;
            let channel_id = msg.channel.id;
            let message_id = msg.id;
            let url = `https://canary.discordapp.com/channels/${guild_id}/${channel_id}/${message_id}`;
            let desc = `[Click to teleport](${url})`;
            if(msg.attachments && msg.attachments.length){
                let attachment = msg.attachments[0];
                let embedtype: 'video' | 'image' = /\.(webm|mp4)$/g.test(attachment.filename) ? 'video' : 'image';
                console.log(embedtype, attachment.filename);
                embed[embedtype] = {
                    url: attachment.url
                };
                
                if (embedtype === 'video') {
                    desc = `[Click to go to video](${url})`;
                }
            } else if (msg.embeds && msg.embeds.length) {
                let nembed = msg.embeds[0];
                if (nembed.video) { 
                    embed.video = nembed.video; 
                    desc = `[Click to go to video](${url})`;
                }
                if (nembed.thumbnail) { embed.thumbnail = nembed.thumbnail; }
                if (nembed.image) { embed.image = nembed.image; }
            }
            if(!embed.description) {
                embed.description = desc;
            } else {
                embed.fields = [{
                    "name": "\u200b",
                    "value": desc
                }];
            }
            this.client.createMessage(pinchannel, { embed: embed });
            return true;
        }
    }

    async connect() {
        this.client.connect();
    }

    die() {
        for (let [guild_id, guild] of this.client.guilds) {
            let server = botparams.servers.ids[guild_id];
            if (!server || this.beta !== server.beta) {
                continue;
            }
            if (server.nickname) {
                guild.editNickname(server.nickname);
            } else {
                guild.editNickname('');
            }
        }
        this.client.disconnect({reconnect: false});
    }
}
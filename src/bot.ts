"use strict"; // Oh boy

import Eris from 'eris';

import { botparams, emojis, Server } from './defines';
import { randomCode, randomEnum, randFromFile, RarityBag, rb_ } from './utils';
import { CommandFunc } from './commands';
import { ClueType, ClueGenerator, mysteryGenerator, clueHelp } from './secrets';
import { createHash } from 'crypto';

type Command = [string, (msg: Eris.Message) => void];

export class Bot {
    client: Eris.Client;

    commands: Command[] = [];
    beta: boolean;

    answer: string = '';
    puzzle_type: ClueType = ClueType.LetterPosition;
    clue_gen?: ClueGenerator;
    clue_count: number = 0;
    last_clue: Date = new Date(0);
    puzzle_id: string = '';
    puzzle_stopped: boolean = false;

    text: { [key: string]: RarityBag } = {};

    owner?: Eris.User;

    constructor(token: string, beta: boolean) {
        this.client = new Eris.Client(token, {
            intents: [
                "guilds",
                "guildMembers",
                "guildVoiceStates",
                "guildPresences",
                "guildMessages",
                "guildMessageReactions",
                "directMessages",
                "directMessageReactions"
            ]
        });

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

    async loadText(): Promise<[boolean, any]> {
        try {
            delete require.cache[require.resolve(`./text.js`)];
            let widget = await import('./text');
            this.text = widget.text;

            return [true, null];
        } catch (e) {
            console.error(e);
            console.log('Could not reload text');
            return [false, e];
        };
    }

    startClues() {
        // if (this.beta) return;

        this.answer = randomCode();
        this.puzzle_type = randomEnum(ClueType);
        this.clue_count = 0;
        this.startGenerator();

        let hasher = createHash('md5');
        hasher.update(this.answer);
        this.puzzle_id = hasher.digest('hex').substr(0, 8);

        this.tellTheBoss(`${this.beta ? 'Beta message! ' : ''}Puzzle started: \`${this.answer}\`. ID: \`${this.puzzle_id}\`. Puzzle type is: \`${ClueType[this.puzzle_type]}\``);
        console.log(`New clue game started: Clue is ${this.answer}. ID is ${this.puzzle_id}. Puzzle type is: ${ClueType[this.puzzle_type]}`);
    }

    startGenerator() {
        this.clue_gen = mysteryGenerator(this.answer, this.puzzle_type);
    }

    canGetClue() {
        return !this.puzzle_stopped && (this.beta || (new Date().getTime() - (1000 * 60 * 60) > this.last_clue.getTime())) && this.clue_gen;
    }

    getClue() {
        if (!this.canGetClue()) {
            console.log("No clue");
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
        let msg = await this.client.createMessage(channel, rb_(this.text.puzzleGenerating, 'Generating clue...'));
        let clue = this.getClue();
        let self = this;

        setTimeout(async function() {
            await msg.edit(`#${++self.clue_count}: \`${clue?.value}\`. Puzzle ID is \`${self.puzzle_id}\``);
            await msg.addReaction(emojis.devil.fullName);
        }, 2500);
    }

    async checkAnswer(answer: string, user: Eris.User) {
        if (!this.answer?.length || !this.owner) {
            return;
        }
        

        if (answer === this.answer) {
            this.answer = '';
            this.clue_gen = undefined;

            let dm = await user.getDMChannel();
            dm.createMessage(rb_(this.text.answerCorrect, 'You got it!'));

            await this.tellTheBoss(`${user.username} (${user.id}) got it!`);

            setTimeout(this.startClues.bind(this), 1000 * 60 * 60 * 24);
        }
    }

    puzzleHelp(): string {
        if (!this.answer) {
            return rb_(this.text.puzzleNothing, 'Nothing going on at the moment');
        } else {
            if (this.puzzle_stopped) {
                return rb_(this.text.puzzleStopped, 'Puzzling has been temporarily stopped');
            } else {
                return `Complete the passphrase and tell it to me for prizes. The clue is: ||${clueHelp(this.puzzle_type)}||\n${this.clue_count} clues have appeared so far\nPuzzle ID is \`${this.puzzle_id}\``;
            }
        }
    }

    reply(msg: Eris.Message, def: string, rb?: RarityBag) {
        return this.client.createMessage(msg.channel.id, rb_(rb, def));    
    }

    replyDM(msg: Eris.Message, def: string, rb?: RarityBag) {
        return msg.author.getDMChannel().then(channel => this.client.createMessage(channel.id, rb_(rb, def)));
    }

    tellTheBoss(what: string) {
        console.log(`[BOSS] ${what}`);
        return this.owner!.getDMChannel().then((ch) => ch.createMessage(what));
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

    async tryRemoveContext(msg: Eris.Message, server: Server) {
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
                randFromFile('nocontext.txt', 'No context', function(name) {
                    (msg.channel as Eris.TextChannel).guild.roles.get(server!.no_context_role)?.edit({name: name});
                });

                if (Math.random() * 4 < 1.0) {
                    this.postClue(server.allowed(msg) ? msg.channel.id : server.allowed_channels[0]);
                }
            }
        }
    }

    async connect() {
        this.loadText();
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
                guild.editNickname(this.client.user.username);
            }
        }
        this.client.disconnect({reconnect: false});
    }
}
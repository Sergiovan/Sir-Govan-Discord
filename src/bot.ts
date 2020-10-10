"use strict"; // Oh boy

import Eris from 'eris';
import Storage from 'node-persist';

import { botparams, emojis, Server } from './defines';
import { sleep, randomCode, randomEnum, randFromFile, RarityBag, rb_ } from './utils';
import { CommandFunc } from './commands';
import { ClueType, ClueGenerator, mysteryGenerator, clueHelp } from './secrets';
import { createHash } from 'crypto';

type Command = [string, (msg: Eris.Message) => void];

const storage_location = 'data/bot';

export class Bot {
    client: Eris.Client;

    commands: Command[] = [];
    beta: boolean;

    answer: string = '';
    puzzle_type: ClueType = ClueType.LetterPosition;
    clue_gen?: ClueGenerator;
    clue_list: string[] = [];
    clue_perm_data: any = null;
    clue_count: number = 0;
    last_clue: Date = new Date(0);
    puzzle_id: string = '';
    puzzle_stopped: boolean = false;

    text: { [key: string]: RarityBag } = {};

    _owner?: Eris.User;

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

        this.load().catch(function(e) {
            console.log('Could not load data from file: ', e);
        }).finally(async function() { 
            await self.owner();
            self.startClues();
        });
    }

    async owner(): Promise<Eris.User> {
        if (this._owner) return this._owner;

        let tries = 60; // 1 minute
        while (tries--) {
            if (this._owner) return this._owner;
            await sleep(1000);
        }

        throw new Error('Owner was not available after 1 minute');
    }

    async load() {
        await Storage.init({ dir: storage_location });
        
        this.answer = await Storage.getItem('answer') ?? '';
        this.puzzle_type = await Storage.getItem('puzzle_type') ?? ClueType.LetterPosition;
        this.clue_list = await Storage.getItem('clue_list') ?? [];
        this.clue_perm_data = await Storage.getItem('clue_perm_data') ?? null;
        this.clue_count = await Storage.getItem('clue_count') ?? 0;
        this.last_clue = new Date(await Storage.getItem('last_clue') ?? 0);
        this.puzzle_id = await Storage.getItem('puzzle_id') ?? '';
        this.puzzle_stopped = await Storage.getItem('puzzle_stopped') ?? false;
    }

    async save() {
        await Storage.init({ dir: storage_location });
        await Promise.all([
            Storage.setItem('answer', this.answer),
            Storage.setItem('puzzle_type', this.puzzle_type),
            Storage.setItem('clue_list', this.clue_list),
            Storage.setItem('clue_perm_data', this.clue_perm_data),
            Storage.setItem('clue_count', this.clue_count),
            Storage.setItem('last_clue', this.last_clue.toJSON()),
            Storage.setItem('puzzle_id', this.puzzle_id),
            Storage.setItem('puzzle_stopped', this.puzzle_stopped)
        ]);
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

        if (this.puzzle_stopped) {
            this.tellTheBoss('Puzzle is paused');
        }

        if (this.answer) { // We already had something going
            this.startGenerator();
            this.tellTheBoss(`${this.beta ? 'Beta message! ' : ''}Puzzle resumed: \`${this.answer}\`. ID: \`${this.puzzle_id}\`. Puzzle type is: \`${ClueType[this.puzzle_type]}\``);
            console.log(`New clue game started: Clue is ${this.answer}. ID is ${this.puzzle_id}. Puzzle type is: ${ClueType[this.puzzle_type]}`);
            
            return;
        }

        // Start from 0
        this.answer = randomCode();
        this.puzzle_type = randomEnum(ClueType);
        this.clue_list = [];
        this.clue_perm_data = null;
        this.clue_count = 0;
        this.startGenerator();

        let hasher = createHash('md5');
        hasher.update(this.answer);
        this.puzzle_id = hasher.digest('hex').substr(0, 8);

        this.tellTheBoss(`${this.beta ? 'Beta message! ' : ''}Puzzle started: \`${this.answer}\`. ID: \`${this.puzzle_id}\`. Puzzle type is: \`${ClueType[this.puzzle_type]}\``);
        console.log(`New clue game started: Clue is ${this.answer}. ID is ${this.puzzle_id}. Puzzle type is: ${ClueType[this.puzzle_type]}`);
    }

    startGenerator() {
        this.clue_gen = mysteryGenerator(this.answer, this.puzzle_type, this.clue_perm_data);
    }

    canGetClue() {
        return !this.puzzle_stopped && (this.beta || (new Date().getTime() - (1000 * 60 * 60) > this.last_clue.getTime())) && this.clue_gen;
    }

    getClue(): string | null {
        if (!this.canGetClue()) {
            console.log("No clue");
            return null;
        }

        if (this.clue_list.length === 0) {
            for (let i = 0; i < 128; ++i) {
                let clue = this.clue_gen!.next();
                if (clue.done) {
                    this.startGenerator();
                    break;
                } else {
                    this.clue_list.push(clue.value.value);
                    this.clue_perm_data = this.clue_perm_data ?? clue.value.perm_data;
                    if (clue.value.cycle_end) {
                        break;
                    }
                }
            }

            console.log("Got me some clues: ", this.clue_list);
        }

        if (this.clue_list.length === 0) {
            console.log("Catastrophic error occurred");
            this.tellTheBoss("Puzzle stopped, catastrophic error happened");
            this.puzzle_stopped = true;
            return 'ERROR';
        }

        let clue = this.clue_list.shift(); 
        this.last_clue = new Date();
        return clue!;
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
            let text = `#${++self.clue_count}: \`${clue}\`. Puzzle ID is \`${self.puzzle_id}\``;
            await msg.edit(text);
            await msg.addReaction(emojis.devil.fullName);
        }, 2500);
    }

    async checkAnswer(answer: string, user: Eris.User) {
        if (!this.answer?.length || !this._owner || this.puzzle_stopped) {
            return;
        }

        if (answer === this.answer) {
            this.answer = '';
            this.puzzle_type = ClueType.LetterPosition;
            this.clue_list = [];
            this.clue_perm_data = null;
            this.clue_count = 0;
            this.clue_gen = undefined;
            this.puzzle_id = '';

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
                return `${rb_(this.text.puzzleGoal, 'Complete the passphrase and tell it to me for prizes')}. ` + 
                       `The clue is: ||${clueHelp(this.puzzle_type)}||\n` + 
                       `${this.clue_count} ${rb_(this.text.puzzleSoFar, 'clues have appeared so far')}\n` + 
                       `Puzzle ID is \`${this.puzzle_id}\``;
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
        console.log(`${'[BOSS]'.cyan} ${what}`);
        return this.owner().then(owner => owner.getDMChannel().then((ch) => ch.createMessage(what)));
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

    async die() {
        for (let [guild_id, guild] of this.client.guilds) {
            let server = botparams.servers.ids[guild_id];
            if (!server || this.beta !== server.beta) {
                continue;
            }

            if (server.nickname) {
                await guild.editNickname(server.nickname);
            } else {
                await guild.editNickname(this.client.user.username);
            }
        }

        let self = this;
        try {
            await this.save(); 
        } catch (e) {
            console.log('Cannot save: ', e);
        } finally {
            self.client.disconnect({reconnect: false});
            process.exit(0);
        }
    }
}
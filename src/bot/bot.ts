import Eris from 'eris';
import * as util from 'util';

import { CommandFunc } from './commands';
import { Puzzler } from './puzzler';
import { BotUser } from './bot_user';
import { cmds, aliases, beta_cmds } from './commands';
import { listeners, fixed_listeners } from './listeners';

import { botparams, emojis, Emoji, Server, xpTransferReason } from '../defines';
import { randFromFile, RarityBag, rb_ } from '../utils';

import { Persist } from '../data/persist';
import { DBWrapper, DBUserProxy } from '../data/db_wrapper';

import { xp } from '../secrets/secrets';

type Command = [string, (msg: Eris.Message) => void];

const storage_location = 'data/node-persist';
const db_location = 'data/db/bot.db';

type CleanupCode = [Date, () => void];

export class Bot {
    client: Eris.Client;
    owner?: Eris.User;
    storage: Persist;
    puzzler: Puzzler = new Puzzler;
    db: DBWrapper;

    users: {[key: string]: BotUser} = {};

    commands: Command[] = [];
    beta: boolean;
    
    cleanup_interval?: NodeJS.Timeout;
    cleanup_list: CleanupCode[] = [];
    message_mutex: Set<string> = new Set();
    
    text: { [key: string]: RarityBag } = {};

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
        this.db = new DBWrapper(db_location);

        const self = this;

        this.setFixedListeners();
        this.setCommands();
        
        // Run every minute
        this.cleanup_interval = setInterval(() => this.run_cleanup(), 1000 * 60); 

        this.storage = new Persist(storage_location);
        this.storage.init().then(() => {
            this.load().catch(function(e) {
                console.log('Could not load data from file: ', e);
            }).finally(async function() { 
                self.startClues();
            });
        });
    }

    setFixedListeners() {
        for (let event in fixed_listeners) {
            this.client.removeAllListeners(event);
            this.client.on(event, fixed_listeners[event].bind(this));
        }
    }

    setListeners() {
        for (let event in listeners) {
            this.client.removeAllListeners(event);
            this.client.on(event, listeners[event].bind(this));
        }
    }

    clearListeners() {
        for (let event in listeners) {
            this.client.removeAllListeners(event);
        }
    }

    setCommands() {
        for (let cmd in cmds) {
            this.addCommand(`!${cmd}`, cmds[cmd]);
        }
        
        for (let alias in aliases) {
            this.addCommand(`!${alias}`, aliases[alias]);
        }
        
        if (this.beta) {
            for (let cmd in beta_cmds) {
                this.addCommand(`!${cmd}`, beta_cmds[cmd]);
            }
        }
    }

    [util.inspect.custom](depth: number, opts: any) {
        const forboden = ['client'];
        const res: any = {};
        for (let prop in this) {
            if (!this.hasOwnProperty(prop) || forboden.indexOf(prop) !== -1) {
                continue;
            } else {
                res[prop] = this[prop];
            }
        }
        return res;
    }

    async load() {
        return Promise.all([
            this.puzzler.load(this.storage)
        ]);
    }

    async save() {
        return Promise.all([
            this.puzzler.save(this.storage),
        ]);
    }

    async run_cleanup(forced: boolean = false) {
        const now = Date.now();
        let i = this.cleanup_list.length;

        while (i--) {
            const [time, fn] = this.cleanup_list[i];
            if (forced || time.getTime() <= now) {
                try {
                    // Can throw, that's fine
                    await fn();
                } catch (e) {
                    // Don't rethrow pls
                    console.log('npm double SIGINT bug?: ', e);
                } finally {
                    this.cleanup_list.splice(i, 1);
                }
            }
        }
    }

    async update_users() {
        const db_users = await this.db.getAllUsers();
        const new_users: {[key: string]: BotUser} = {};
        const seen: Set<string> = new Set<string>();

        for (let user of db_users) {
            seen.add(user.id);
            if (this.client.users.has(user.id)) {
                user.is_member = 1;

                new_users[user.id] = new BotUser(user);

                user.commit();
            } else {
                user.is_member = 0;
                user.commit();
            }
        }

        for (let [guild_id, guild] of this.client.guilds) {
            const server = botparams.servers.ids[guild_id.toString()];

            if (server && server.beta === this.beta) {
                for (let [member_id, member] of guild.members) {
                    if (seen.has(member_id.toString())) {
                        const db_user = new_users[member_id.toString()];
                        db_user.update_member(member);

                        db_user.commit();
                    } else {
                        const db_user = await this.db.addUser(member.user, 1, member.bot ? 1 : 0 , member.nick);
                        new_users[member.id] = new BotUser(db_user);
                    }
                }
            }     
        }

        this.users = new_users;
    }

    add_user(user: DBUserProxy) {
        this.users[user.id] = new BotUser(user);
    }

    /// Notice, this method of locking/unlocking only works because 
    /// this app is single-threaded...
    message_locked(msg: Eris.Message) {
        return this.message_mutex.has(msg.id);
    }
    
    lock_message(msg: Eris.Message, ms: number = 1000 * 60) {
        this.message_mutex.add(msg.id);
        this.add_cleanup_task(() => this.message_mutex.delete(msg.id), ms);
    }

    add_cleanup_task(task: () => void, delay_ms: number) {
        if (this.cleanup_interval) {
            this.cleanup_list.push([
                new Date(Date.now() + delay_ms),
                task
            ]);
        } else {
            console.log('Forced task through');
            task();
        }
    }

    parse(msg: Eris.Message) {
        const message = msg.content;
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

    async loadText(): Promise<[boolean, any]> {
        try {
            delete require.cache[require.resolve(`../secrets/text.js`)];
            const widget = await import('../secrets/text');
            this.text = widget.text;

            return [true, null];
        } catch (e) {
            console.error(e);
            console.log('Could not reload text');
            return [false, e];
        };
    }

    async startClues() {
        const text = `${this.beta ? 'Beta message! ' : ''}${this.puzzler.startClues()}`;
        console.log(text);
        this.tellTheBoss(text); // This will not do anything if we're not connected by this point

        const pzl = await this.db.getPuzzle(this.puzzler.puzzle_id);
        if (!pzl) {
            this.db.addPuzzle({
                id: this.puzzler.puzzle_id,
                answer: this.puzzler.answer,
                type: this.puzzler.puzzle_type as number,
                started_time: new Date()
            });
        }
    }

    async postClue(channel: string, forced: boolean = false) {
        console.log('Posting clue');
        let clue: string | null;

        try {
            clue = this.puzzler.getClue(forced);
        } catch (e) {
            this.tellTheBoss(e.message);
            console.error(e.message);
            return;
        }

        if (clue === null) {
            return;
        }

        let msg = await this.client.createMessage(channel, rb_(this.text.puzzleGenerating, 'Generating clue...'));

        const text = `#${this.puzzler.clue_count}: \`${clue}\`. Puzzle ID is \`${this.puzzler.puzzle_id}\``;
        console.log(`Clue: ${text}`);

        setTimeout(async () => {
            msg = await msg.edit(text);
            await this.db.addClue(this.puzzler.puzzle_id, msg);
            await msg.addReaction(emojis.devil.fullName);
        }, 2500);
    }

    async checkAnswer(answer: string, user: Eris.User) {
        if (!this.owner || (user.id === this.owner.id && !this.beta)) {
            return;
        }

        if (this.puzzler.checkAnswer(answer)) {
            const id = this.puzzler.puzzle_id;
            this.puzzler.endPuzzle();
            const dm = await user.getDMChannel();
            dm.createMessage(rb_(this.text.answerCorrect, 'You got it!'));

            await this.tellTheBoss(`${user.username} (${user.id}) got it!`);

            setTimeout(this.startClues.bind(this), 1000 * 60 * 60 * 24);

            const puzzle = await this.db.getPuzzle(id);
            if (puzzle && this.users[user.id]) {
                puzzle.winner = this.users[user.id].db_user.rowid;
                puzzle.ended_time = new Date();
                puzzle.commit();
            }
        }
    }

    puzzleHelp(): string {
        const [puzzle_active, puzzle_stopped, help] = this.puzzler.getHelp();
        if (!puzzle_active) {
            return rb_(this.text.puzzleNothing, 'Nothing going on at the moment');
        } else {
            if (puzzle_stopped) {
                return rb_(this.text.puzzleStopped, 'Puzzling has been temporarily stopped');
            } else {
                return `${rb_(this.text.puzzleGoal, 'Complete the passphrase and tell it to me for prizes')}. ` + 
                       `The clue is: ||${help}||\n` + 
                       `${this.puzzler.clue_count} ${rb_(this.text.puzzleSoFar, 'clues have appeared so far')}\n` + 
                       `Puzzle ID is \`${this.puzzler.puzzle_id}\``;
            }
        }
    }

    transferXp(amount: number, from: Eris.User, to: Eris.User, reason: xpTransferReason): boolean;
    transferXp(amount: number, from: Eris.User | null, to: Eris.User, reason: xpTransferReason): true;
    transferXp(amount: number, from: Eris.User, to: Eris.User | null, reason: xpTransferReason): boolean;
    transferXp(amount: number, from: Eris.User | null, to: Eris.User | null, reason: xpTransferReason) {
        if (from) {
            if (!this.users[from.id].remove_xp(amount)) {
                return false;
            }
            this.users[from.id].commit()
        }
        if (to) {
            this.users[to.id].add_xp(amount);
            this.users[to.id].commit();
        }
        this.db.transferXP(from, to, amount, reason as number);
        return true;
    }

    async tickUser(user: Eris.User) {
        const bot_user = this.users[user.id];
        if ((bot_user.last_spoke + xp.passive_timeout * 1000) < Date.now()) {
            bot_user.last_spoke = Date.now();
            this.transferXp(xp.secondsOfXp(xp.passive_timeout), null, user, xpTransferReason.Passive);
        }
    }

    reply(msg: Eris.Message, def: string, rb?: RarityBag) {
        return this.client.createMessage(msg.channel.id, rb_(rb, def));    
    }

    async replyDM(msg: Eris.Message, def: string, rb?: RarityBag) {
        const channel = await msg.author.getDMChannel();
        return await this.client.createMessage(channel.id, rb_(rb, def));
    }

    async tellTheBoss(what: string) {
        console.log(`${'[BOSS]'.cyan} ${what}`);
        const ch = this.owner?.getDMChannel();
        return (await ch)?.createMessage(what);
    }

    async maybe_pin(msg: Eris.Message, emoji: Emoji) {
        const findname = emoji.id ? `${emoji.name}:${emoji.id}` : emoji.name;
        if (msg.author.bot) {
            return;
        }
        if ((msg.reactions[emojis.pushpin.fullName] && 
            msg.reactions[emojis.pushpin.fullName].me) ||
            this.message_locked(msg)) {
            return;
        }

        let reactionaries = await msg.getReaction(findname, 4);
        if(reactionaries.filter((user) => user.id !== msg.author.id).length >= 3){
            //We pin that shit!
            this.lock_message(msg);
            msg.addReaction(emojis.pushpin.fullName);
            this.pin(msg);
        }
    }

    pin(msg: Eris.Message, forced: boolean = false) {
        const server = botparams.servers.getServer(msg);
        const pinchannel = server?.pin_channel;
        if (!pinchannel) {
            console.log("Can't pin this >:(");
            return false;
        } else {
            const icon = forced ? 
                'https://emojipedia-us.s3.amazonaws.com/thumbs/120/twitter/131/double-exclamation-mark_203c.png' : 
                'https://cdn.discordapp.com/emojis/263774481233870848.png';
            const r = Math.floor(Math.random() * 0x10) * 0x10;
            const g = Math.floor(Math.random() * 0x10) * 0x10;
            const b = Math.floor(Math.random() * 0x10) * 0x10;
            const embed: Eris.Embed = {
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
            const guild_id = server!.id;
            const channel_id = msg.channel.id;
            const message_id = msg.id;
            const url = `https://canary.discordapp.com/channels/${guild_id}/${channel_id}/${message_id}`;
            let desc = `[Click to teleport](${url})`;
            if(msg.attachments && msg.attachments.length){
                const attachment = msg.attachments[0];
                const embedtype: 'video' | 'image' = /\.(webm|mp4)$/g.test(attachment.filename) ? 'video' : 'image';
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

    async maybe_steal(msg: Eris.Message, user: Eris.User) {
        if (!msg.reactions[emojis.devil.fullName].me ||
            this.message_locked(msg)) {
            return;
        }

        this.lock_message(msg);
        const content = msg.content!;
        await msg.removeReaction(emojis.devil.fullName);
        await msg.edit(`${rb_(this.text.puzzleSteal, 'Stolen')} by ${user.username}`);

        (await user.getDMChannel()).createMessage(content);
        this.db.addClueSteal(msg, user);
        this.add_cleanup_task(() => msg.delete(), 1000 * 5 * 60);
    }

    async tryRemoveContext(msg: Eris.Message, server: Server) {
        const channel = server.no_context_channel;
        if (msg.cleanContent?.length && msg.cleanContent.length <= 280 && !msg.attachments.length) {
            this.client.createMessage(channel, msg.cleanContent);
            this.transferXp(xp.secondsOfXp(60 * 60), null, msg.author, xpTransferReason.NoContext);
            if (server.no_context_role) {
                for (let [_, member] of (msg.channel as Eris.TextChannel).guild.members) {
                    if (member.id === msg.author.id) {
                        member.addRole(server.no_context_role);
                    } else if (member.roles.includes(server.no_context_role)) {
                        member.removeRole(server.no_context_role);
                    }
                }
                randFromFile('nocontext.txt', 'No context', function(name) {
                    (msg.channel as Eris.TextChannel).guild.roles.get(server.no_context_role)?.edit({name: name});
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
        try {
            if (this.cleanup_interval) {
                clearInterval(this.cleanup_interval);
            }

            await this.save(); 

            await this.run_cleanup(true);

            for (let [guild_id, guild] of this.client.guilds) {
                const server = botparams.servers.ids[guild_id];
                if (!server || this.beta !== server.beta) {
                    continue;
                }

                if (server.nickname) {
                    await guild.editNickname(server.nickname);
                } else {
                    await guild.editNickname(this.client.user.username);
                }
            }

        } catch (e) {
            console.log('Error while quitting: ', e);
        } finally {
            this.client.disconnect({reconnect: false});
            this.db.close();
        }
    }
}
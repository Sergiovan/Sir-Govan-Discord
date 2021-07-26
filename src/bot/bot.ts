import * as D from 'discord.js';
import * as util from 'util';

import { encode } from 'html-entities';
import * as twemoji from 'twemoji';

import { CommandFunc } from './commands';
import { Puzzler } from './puzzler';
import { BotUser } from './bot_user';
import { cmds, aliases, beta_cmds } from './commands';
import { listeners, fixed_listeners, ListenerFunction } from './listeners';

import { emojis, Emoji, JsonableServer, Server, xpTransferReason } from '../defines';
import { randFromFile, RarityBag, rb_, Logger } from '../utils';

import { Persist } from '../data/persist';
import { DBWrapper, DBUserProxy } from '../data/db_wrapper';

import { xp } from '../secrets/secrets';
import { createImage, TweetData, TweetMoreData, TweetTheme } from '../twitter';

/** Pair containing command name and command function */
type Command = [string, CommandFunc];

const storage_location = 'data/node-persist';
const db_location = 'data/db/bot.db';

/** Pair containing a function and when to execute it */
type CleanupCode = [Date, () => void];

/** Wrapper over Discord.js Client class */
export class Bot {
    client: D.Client; // Discord.js client 
    token: string; // Login token
    owner?: D.User; // Me, Sergiovan. How exciting
    storage: Persist; // Basic persistent storage
    puzzler: Puzzler = new Puzzler; // Puzzle stuff
    db: DBWrapper; // Database for advanced persistent storage

    users: {[key: string]: BotUser | undefined} = {}; // A map from User ID to internal user representation

    commands: Command[] = []; // A list of all the commands available
    beta: boolean; // If the bot is running in beta mode
    
    cleanup_interval?: NodeJS.Timeout; // Interval for cleanup functions
    cleanup_list: CleanupCode[] = []; // List of to-be-cleaned-up actions
    message_mutex: Set<string> = new Set(); // Message lock
    
    text: { [key: string]: RarityBag } = {}; // Text instance, for random chance texts

    servers: { [key: string]: Server } = {};
    ownerID: D.Snowflake = '120881455663415296'; // Sergiovan#0831
    #server_loaded: boolean = false;

    constructor(token: string, beta: boolean) {
        this.token = token;

        const Flags = D.Intents.FLAGS;
        this.client = new D.Client({
            intents: [
                Flags.GUILDS,
                Flags.GUILD_MEMBERS,
                Flags.GUILD_VOICE_STATES,
                Flags.GUILD_PRESENCES,
                Flags.GUILD_MESSAGES,
                Flags.GUILD_MESSAGE_REACTIONS,
                Flags.DIRECT_MESSAGES,
                Flags.DIRECT_MESSAGE_REACTIONS
            ],
            partials: [
                'MESSAGE',
                'CHANNEL',
                'REACTION'
            ]
        });

        this.beta = beta;
        this.db = new DBWrapper(db_location);

        const self = this;

        this.setFixedListeners();
        this.setCommands();
        
        // Run every minute
        this.cleanup_interval = setInterval(() => this.run_cleanup(), 1000 * 60); // Cleanup runs every minute

        // Initialize storage and then load it too
        this.storage = new Persist(storage_location);
        this.storage.init().then(() => {
            this.load().catch(function(e) {
                Logger.error('Could not load data from file: ', e);
            }).finally(async function() { 
                self.startClues(); // After it's loaded we start the puzzle
            });
        });
    }

    /** Only called in the constructor, these are set just once */
    setFixedListeners() {
        let event: keyof D.ClientEvents;
        for (event in fixed_listeners) {
            this.client.removeAllListeners(event);
            if (!fixed_listeners[event]) continue;
            let func: ListenerFunction = fixed_listeners[event]!;
            this.client.on(event, func.bind(this));
        }
    }

    /** Called every time the bot restarts */
    setListeners() {
        let event: keyof D.ClientEvents;
        for (event in listeners) {
            this.client.removeAllListeners(event);
            if (!listeners[event]) continue;
            let func: ListenerFunction = listeners[event]!;
            this.client.on(event, func.bind(this));
        }
    }

    /** Removes clearable listeners */
    clearListeners() {
        for (let event in listeners) {
            this.client.removeAllListeners(event);
        }
    }

    /** Sets up commands and aliases */
    setCommands() {
        // TODO Slash commands
        for (let cmd in cmds) {
            this.addCommand(`!${cmd}`, cmds[cmd]);
        }
        
        for (let alias in aliases) {
            this.addCommand(`!${alias}`, aliases[alias]);
        }
        
        // These commands are for debugging only
        if (this.beta) {
            for (let cmd in beta_cmds) {
                this.addCommand(`!${cmd}`, beta_cmds[cmd]);
            }
        }
    }

    /** Custom inspect function for util.inspect */
    [util.inspect.custom](depth: number, opts: any) {
        const forboden = ['client']; // We're not gonna look into these properties
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

    /** Loads all basic permanent storage */
    async load() {
        return Promise.all([
            this.puzzler.load(this.storage)
        ]);
    }

    /** Saves all basic permanent storage */
    async save() {
        return Promise.all([
            this.puzzler.save(this.storage),
            this.save_servers()
        ]);
    }

    load_servers() {
        let self = this;
        return this.storage.get('servers', {}).then((serv: { [key: string]: JsonableServer }) => {
            for (let id in serv) {
                this.servers[id] = new Server(this.client, id as D.Snowflake, serv[id]);
            }
            self.#server_loaded = true;
        });
    }

    save_servers() {
        if (this.#server_loaded) {
            const servers: { [key: string]: JsonableServer } = {};
            for (let id in this.servers) {
                servers[id] = this.servers[id].as_jsonable();
            }
            
            return this.storage.set('servers', servers);
        }
    }

    get_server(o: D.Message | D.GuildChannel | D.ThreadChannel | D.Guild | D.GuildMember) {
        let ret: Server | undefined;
        if (o instanceof D.Message) {
            ret = o.guild?.id ? this.servers[o.guild.id] : undefined;
        } else if (o instanceof D.Guild) {
            ret = this.servers[o.id];
        } else {
            ret = this.servers[o.guild.id];
        }

        if (!ret) return ret;
        if (ret._beta !== this.beta) return null;
        return ret;
    }

    #channelize(on: D.Message | D.GuildChannel | D.ThreadChannel) {
        let channel: D.GuildChannel | D.ThreadChannel;
        
        if (on instanceof D.Message) {
            if (on.channel instanceof D.DMChannel) {
                return true;
            } else if (on.channel instanceof D.NewsChannel) {
                return false;
            }
            channel = on.channel;
        } else if (!on.isText()) { 
            return false;
        } else {
            channel = on;
        }
        if (channel instanceof D.ThreadChannel) {
            if (!channel.parent || channel.parent instanceof D.NewsChannel) {
                return false;
            }
            channel = channel.parent;
        }
        return channel;
    }

    // Bot can listen to commands and reply
    can_command(on: D.Message | D.GuildChannel | D.ThreadChannel): boolean {
        let channel = this.#channelize(on);
        if (channel === true || channel === false) return channel;
        
        const server = this.get_server(on);
        if (!server) return false;
        const perms = channel.permissionsFor(channel.guild.me!);
        const f = D.Permissions.FLAGS;
        return perms.has(f.VIEW_CHANNEL | f.SEND_MESSAGES) && server.allowed_commands(channel);
    }

    // Bot can send messages and listen
    can_talk(on: D.Message | D.GuildChannel | D.ThreadChannel): boolean {
        let channel = this.#channelize(on);
        if (channel === true || channel === false) return channel;

        const server = this.get_server(on);
        if (!server) return false;
        const perms = channel.permissionsFor(channel.guild.me!);
        const f = D.Permissions.FLAGS;
        return perms.has(f.VIEW_CHANNEL | f.SEND_MESSAGES);
    }

    // Bot can listen
    can_listen(on: D.Message | D.GuildChannel | D.ThreadChannel): boolean {
        let channel = this.#channelize(on);
        if (channel === true || channel === false) return channel;

        const server = this.get_server(on);
        if (!server) return false;
        const perms = channel.permissionsFor(channel.guild.me!);
        const f = D.Permissions.FLAGS;
        return perms.has(f.VIEW_CHANNEL | f.READ_MESSAGE_HISTORY) && server.allowed_listen(channel);
    }

    /** Goes through all cleanup code
     * 
     * When `forced` is true, all the actions are called regardless of the date requirement
     */
    async run_cleanup(forced: boolean = false) {
        const now = Date.now();
        let i = this.cleanup_list.length;

        // We go through the list in reverse because we're removing elements
        while (i--) {
            const [time, fn] = this.cleanup_list[i];
            if (forced || time.getTime() <= now) {
                try {
                    // Can throw, that's fine
                    await fn();
                } catch (e) {
                    // Don't rethrow pls
                    Logger.error('npm double SIGINT bug?: ', e);
                } finally {
                    this.cleanup_list.splice(i, 1);
                }
            }
        }
    }

    /** Updates the user database with all users the bot can see */
    async update_users() {
        const db_users = await this.db.getAllUsers(); // Users in the database
        const new_users: {[key: string]: BotUser} = {}; // Users that are new
        const seen: Set<D.Snowflake> = new Set<D.Snowflake>(); // Users we have already seen

        for (let user of db_users) {
            seen.add(user.id);
            if (this.client.users.resolve(user.id)) { // The bot can see this user
                user.is_member = 1;

                new_users[user.id] = new BotUser(user);

                user.commit();
            } else { // The bot cannot see this user anymore, ignore it
                user.is_member = 0;
                user.commit();
            }
        }

        // For each member in every server
        for (let [guild_id, guild] of this.client.guilds.cache) {
            const server = this.servers[guild_id];

            if (server && server._beta === this.beta) {
                for (let [member_id, member] of guild.members.cache) {
                    if (seen.has(member_id)) { // We've seen this user when going through the db
                        const db_user = new_users[member_id];
                        db_user.update_member(member); // Update its member data as well

                        db_user.commit();
                    } else {
                        // This user was not in the db, so add it now
                        const db_user = await this.db.addUser(member.user, 1, member.user.bot ? 1 : 0 , member.nickname);
                        new_users[member.id] = new BotUser(db_user);
                    }
                }
            }     
        }

        this.users = new_users;
    }

    /** Add a user to the internal cache */
    add_user(user: DBUserProxy) {
        this.users[user.id] = new BotUser(user);
    }

    async get_or_add_user(user: D.User) {
        const usr = this.users[user.id];
        if (usr) {
            return usr;
        } else {
            const db_user = await this.db.addUser(user, 1, user.bot ? 1 : 0);
            return this.users[user.id] = new BotUser(db_user);
        }
    }

    /** Check if a certain message is locked 
     * 
     * Notice, this method of locking/unlocking only works because 
     * this app is single-threaded...
    */
    message_locked(msg: D.Message) {
        // TODO This mechanism has to be rethought
        return this.message_mutex.has(msg.id);
    }
    
    /** Lock a message, unlock after a minute */
    lock_message(msg: D.Message, ms: number = 1000 * 60) {
        // TODO Change this mechanism
        this.message_mutex.add(msg.id);
        this.add_cleanup_task(() => this.message_mutex.delete(msg.id), ms);
    }

    /** Adds a cleanup task to the list, to be done after `delay_ms` */
    add_cleanup_task(task: () => void, delay_ms: number) {
        if (this.cleanup_interval) {
            this.cleanup_list.push([
                new Date(Date.now() + delay_ms),
                task
            ]);
        } else {
            Logger.debug('Forced task through');
            task();
        }
    }

    /** Parses a message to run a command */
    parse(msg: D.Message) {
        // TODO Commands...
        const message = msg.content;
        for(let [commandName, command] of this.commands){
            if(message.split(' ')[0] === commandName){
                command.call(this, msg);
                return true;
            }
        }
        return false;
    }

    /** Adds a command to the list of commands */
    addCommand(name: string, command: CommandFunc) {
        // TODO Commands...
        this.commands.push([name, command]);
    }

    /** Loads or reloads the internal text instance */
    async loadText(): Promise<[boolean, any]> {
        try {
            // Wonky node.js reloading of modules
            delete require.cache[require.resolve(`../secrets/text.js`)];
            const widget = await import('../secrets/text'); // This is ts?
            this.text = widget.text;

            return [true, null];
        } catch (e) {
            Logger.error(e);
            Logger.error('Could not reload text');
            return [false, e];
        };
    }

    /** Begin the puzzle */
    async startClues() {
        const text = `${this.beta ? 'Beta message! ' : ''}${this.puzzler.startClues()}`;
        this.tellTheBoss(text); // This will not do anything if we're not connected by this point

        // Loads the latest from the database, or creates a new one
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

    /** Posts a single clue from the puzzle to a channel
     * 
     * If `forced` is true the clue is forced
     */
    async postClue(channel: D.Snowflake, forced: boolean = false) {
        let chn = this.client.channels.resolve(channel);
        if (!chn || !chn.isText()) {
            Logger.error(`Tried to post clue in invalid channel ${channel}`);
            return;
        }
        
        Logger.debug('Posting clue');
        let clue: string | null;

        try {
            clue = this.puzzler.getClue(forced);
        } catch (e) {
            this.tellTheBoss(e.message);
            Logger.error(e.message);
            return;
        }

        if (clue === null) {
            return; // Can happen if it's not time yet
        }

        let msg = await chn.send(rb_(this.text.puzzleGenerating, 'Generating clue...'));

        const text = `#${this.puzzler.clue_count}: \`${clue}\`. Puzzle ID is \`${this.puzzler.puzzle_id}\``;
        Logger.debug(`Clue: ${text}`);

        // TODO Funky buttons?
        setTimeout(async () => {
            msg = await msg.edit(text);
            await this.db.addClue(this.puzzler.puzzle_id, msg);
            await msg.react(emojis.devil.toString());
        }, 2500);
    }

    /** Verifies that `answer` is the answer to the current puzzle */
    async checkAnswer(answer: string, user: D.User) {
        // I cannot accidentally ruin the puzzle... Haha oops
        if (!this.owner || (user.id === this.owner.id && !this.beta)) {
            return;
        }

        // If the answer is correct
        if (this.puzzler.checkAnswer(answer)) {
            const id = this.puzzler.puzzle_id;
            this.puzzler.endPuzzle();
            const dm = user.dmChannel ?? await user.createDM();
            dm.send(rb_(this.text.answerCorrect, 'You got it!'));

            await this.tellTheBoss(`${user.username} (${user.id}) got it!`);

            // Start a new puzzle in an hour
            this.client.setTimeout(this.startClues.bind(this), 1000 * 60 * 60 * 24);

            const puzzle = await this.db.getPuzzle(id);
            if (puzzle && this.users[user.id]) {
                puzzle.winner = (await this.get_or_add_user(user)).db_user.rowid;
                puzzle.ended_time = new Date();
                puzzle.commit();
            }
        }
    }

    /** Get a help string from the puzzle */
    puzzleHelp(): string {
        const [puzzle_active, puzzle_stopped, help] = this.puzzler.getHelp();
        if (!puzzle_active) { // No puzzle
            return rb_(this.text.puzzleNothing, 'Nothing going on at the moment');
        } else {
            if (puzzle_stopped) { // Puzzle, but currently not going
                return rb_(this.text.puzzleStopped, 'Puzzling has been temporarily stopped');
            } else { // Actual clue
                return `${rb_(this.text.puzzleGoal, 'Complete the passphrase and tell it to me for prizes')}. ` + 
                       `The clue is: ||${help}||\n` + 
                       `${this.puzzler.clue_count} ${rb_(this.text.puzzleSoFar, 'clues have appeared so far')}\n` + 
                       `Puzzle ID is \`${this.puzzler.puzzle_id}\``;
            }
        }
    }

    /** Transfer `amount` xp between two users. 
     * 
     * If `from` is null, the xp is created from thin air
     * 
     * If `to` is null, the xp is destroyed
     * */
    transferXp(amount: number, from: D.User, to: D.User, reason: xpTransferReason): boolean;
    transferXp(amount: number, from: D.User | null, to: D.User, reason: xpTransferReason): true;
    transferXp(amount: number, from: D.User, to: D.User | null, reason: xpTransferReason): boolean;
    transferXp(amount: number, from: D.User | null, to: D.User | null, reason: xpTransferReason) {
        if (from) {
            if (!this.users[from.id]!.remove_xp(amount)) {
                return false; // If we cannot remove this amount of xp from the donor, the whole thing is cancelled
            }
            this.users[from.id]!.commit()
        }
        if (to) {
            this.users[to.id]!.add_xp(amount);
            this.users[to.id]!.commit();
        }
        this.db.transferXP(from, to, amount, reason as number);
        return true;
    }

    /** Transfer passive xp to a user when they talk */
    async tickUser(user: D.User) {
        const bot_user = await this.get_or_add_user(user);

        if ((bot_user.last_spoke + xp.passive_timeout * 1000) < Date.now()) {
            bot_user.last_spoke = Date.now();
            this.transferXp(xp.secondsOfXp(xp.passive_timeout), null, user, xpTransferReason.Passive);
        }
    }

    /** Returns a cleaned string from a message content. Spaces in names are replaced with \0
     * and need to be returned to ' '
    */
    clean_content(msg: D.Message | string, channel: D.TextChannel | D.ThreadChannel): string {
        let text = typeof msg === 'string' ? msg : msg.content;
        let self = this;

        text = text.replace(/<@!?([0-9]+)>/g, function(match: string, m1: D.Snowflake) {
            const m = channel.guild.members.resolve(m1);
            return `@${m?.displayName ?? self.client.users.resolve(m1)?.username ?? "unknown-user"}`.replace(/ /g, '\x00');
        });

        text = text.replace(/<@\&([0-9]+)>/g, function(match: string, m1: D.Snowflake) {
            const r = channel.guild.roles.resolve(m1);
            return `#${r?.name ?? "deleted-role"}`.replace(/ /g, '\x00');
        });

        text = text.replace(/<#([0-9]+)>/g, function(match: string, m1: D.Snowflake) {
            const c = channel.guild.channels.resolve(m1);
            return `@${c?.name ?? "deleted-channel"}`.replace(/ /g, '\x00');
        });

        return text;
    }

    async randomize_self() {
        let promises = [];
        for (let [guild_id, guild] of this.client.guilds.cache) {
            const server = this.servers[guild_id];
            if (!server || this.beta !== server._beta) {
                continue;
            }
            const new_nick = rb_(this.text.nickname, server.nickname || 'Sir Govan') + (this.beta ? ' (β)' : '');
            if (!guild.me) {
                continue;
            }
            Logger.debug(`Setting nickname to "${new_nick}" in ${guild.name}`);
            promises.push(guild.me.setNickname(new_nick));
        }

        // LISTENING: Listening to
        // WATCHING: Watching
        // PLAYING: Playing
        // STREAMING: Playing (But the profile says "Live on xxx")
        // COMPETING: Competing in
        const doing = rb_(this.text.status_type, '') as 'LISTENING' | 'WATCHING' | 'PLAYING' | 'STREAMING' | 'COMPETING' | '';
        if (doing !== '') {
            const texts = {
                LISTENING: this.text.status_listening,
                WATCHING: this.text.status_watching,
                PLAYING: this.text.status_playing,
                STREAMING: this.text.status_watching,
                COMPETING: this.text.status_competing
            };

            this.client.user!.setActivity(rb_(texts[doing], 'something'), {type: doing});
        } else {
            this.client.user!.setActivity();
        }
        // this.client.user!.setActivity('testing', {type: 'COMPETING'});
        return Promise.all(promises);
    }

    /** Writes a message on the same channel as `msg`
     * 
     * NOTE: Does not use inline replies!! 
     */
    reply(msg: D.Message, def: string, rb?: RarityBag) {
        return msg.channel.send(rb_(rb, def));
    }

    /** Writes a DM to the author of `msg` */
    async replyDM(msg: D.Message, def: string, rb?: RarityBag) {
        const channel = msg.author.dmChannel ?? await msg.author.createDM();
        return await channel.send(rb_(rb, def));
    }

    /** Writes me, Sergiovan, a DM */
    async tellTheBoss(what: string) {
        Logger.debug(`${'[BOSS]'.cyan} ${what}`);
        const ch = this.owner?.dmChannel ?? await this.owner?.createDM();
        return ch?.send(what);
    }

    /** Attempts to pin a message */
    async maybe_pin(msg: D.Message, emoji: Emoji, to?: D.TextChannel | null, pinmoji: Emoji = emoji) {
        const server = this.get_server(msg);
        if (!server || !to || !this.can_talk(to)) {
            Logger.error(`Cannot pin to ${to}`);
            return;
        }

        if (msg.author.id === this.client.user!.id) { // Do not pin bot messages
            return;
        }

        // Do not pin messages where I've already reacted
        const reactions = msg.reactions.resolve(emoji.to_reaction_resolvable());
        if (!reactions || reactions.me || this.message_locked(msg)) {
            return; // If this messages has been pinned or is locked for pinning, cease
        }

        let reactionaries = await reactions.users.fetch();
        if (!reactionaries) return; // ???

        // At least `server.pin_amount` pins that are not the author or bots
        if(reactionaries.filter((user) => user.id !== msg.author.id && !user.bot).size >= server.pin_amount){
            //We pin that shit!
            this.lock_message(msg); // TODO huuuuu
            await msg.react(emoji.toString());
            this.pin(msg, to, pinmoji);
        }
    }

    /** Attempts to retweet a message */
    async maybe_retweet(msg: D.Message, retweeter: D.GuildMember, add_extras: boolean) {
        let self = this;

        function random_tweet_number() {
            let rand = Math.random() - 0.25;
            if (rand < 0) return 0;
            rand *= (1 / 0.75);
            rand += 1;
            rand **= 13.2875681028;
            return Math.floor(rand);
        }

        function number_to_twitter_text(num: number, sym: string) {
            if (!num) {
                return '';
            } else if (!sym) {
                return num.toString();
            } else {
                let str = num.toString().substr(0, 4);
                if (str.length !== 4 || str[str.length - 1] === '0') {
                    return `${str.substr(0, 3)}${sym}`;
                } else {
                    return `${str.substr(0, str.length - 1)}.${str[str.length - 1]}${sym}`;
                }
            }
        }

        function clean_content(text: string) {
            text = encode(text);

            text = text.replace(/\n/g, '<br>');
            text = text.replace(/(@[^ \t\n\r]+)/g, '<span class="twitter-link">$1</span>');
            text = text.replace(/(#[^ \t\n\r]+)/g, '<span class="twitter-link">$1</span>');
            text = text.replace(/(https?:\/\/[^ \t\n\r]+)/g, '<span class="twitter-link">$1</span>');

            text = text.replace(/\x00/g, ' ');

            return text;
        }

        function emojify(text: string) {
            text = twemoji.parse(text, {
                callback: function(icon, options: any, variant) {
                    switch ( icon ) {
                        case 'a9':      // © copyright
                        case 'ae':      // ® registered trademark
                        case '2122':    // ™ trademark
                            return false;
                    }
                    return ''.concat(options.base, options.size, '/', icon, options.ext);
                }
            });
            text = text.replace(/&lt;a?\:.*?\:([0-9]+)&gt;/g, '<img class="emoji" src="https://cdn.discordapp.com/emojis/$1.png">');
            return text;
        }

        function is_reply(msg: D.Message): boolean {
            return msg.reference !== null;
        }

        async function replies_at(msg: D.Message) {
            return msg.reference ? (await msg.fetchReference()).author.tag : '';
        }

        function get_attachment(msg: D.Message): string {
            if (msg.attachments.size) {
                for (let [att_id, att] of msg.attachments) {
                    if (att.name && !/\.(webm|mp4)$/g.test(att.name)) { // Img
                        return att.url;
                    }
                }
            }
            if (msg.embeds.length) {
                for (let embed of msg.embeds) {
                    if (embed.image) {
                        return embed.image.url;
                    }
                }
            }
            return '';
        }

        function group_messages(msgs: D.Message[]): D.Message[][] {
            let ret: D.Message[][] = [];
            let cur: D.Message[] = [msgs[0]];
            let last_user: string = msgs[0].author.id;
            let last_time: number = msgs[0].createdTimestamp;
            const time_diff = 1000 * 30; // 30 seconds
            let has_attachment: boolean = get_attachment(msgs[0]) !== '';

            for (let i = 1; i < msgs.length; ++i) {
                let msg = msgs[i];

                if (msg.author.id !== last_user) {
                    ret.push(cur);
                    cur = [msg];
                    has_attachment = get_attachment(msg) !== '';
                    last_user = msg.author.id;
                    last_time = msg.createdTimestamp;
                    continue;
                }
                
                // Same user

                if (msg.createdTimestamp > last_time + time_diff ) {
                    ret.push(cur);
                    cur = [msg];
                    has_attachment = get_attachment(msg) !== '';
                    last_time = msg.createdTimestamp;
                    continue;
                }

                last_time = msg.createdTimestamp;

                // In time

                if (is_reply(msg)) {
                    ret.push(cur);
                    cur = [msg];
                    has_attachment = get_attachment(msg) !== '';
                    continue;
                }

                if (get_attachment(msg)) {
                    let had_attachment = has_attachment;
                    has_attachment = true;
                    if (had_attachment) {
                        ret.push(cur);
                        cur = [msg];
                        continue;
                    } 
                }

                cur.push(msg);
            }

            ret.push(cur);
            return ret;
        }

        const server = this.get_server(msg);
        if (!server || server._beta !== this.beta || !server.hof_channel) {
            return;
        }

        if (msg.channel instanceof D.DMChannel || msg.channel instanceof D.NewsChannel) return; // No DMs or... news... channels?

        const emoji = add_extras ? emojis.repeat.toString() : emojis.repeat_one.toString();

        // TODO Message locking... huuuu...
        if (msg.reactions.resolve(emoji)?.me || this.message_locked(msg)) {
            return; // If this messages has been pinned or is locked for pinning, cease
        }

        this.lock_message(msg);
        await msg.react(emoji);
        this.add_cleanup_task(() => {
            msg.reactions.resolve(emoji)?.users.remove(self.client.user!.id);
        }, 1000 * 60 * 30);

        const channel = msg.channel;
        const guild = channel.guild;

        const author = msg.author;
        const author_member = await guild.members.fetch(author.id);

        const months = [
            'Jan', 'Feb', 'Mar',
            'Apr', 'May', 'Jun',
            'Jul', 'Aug', 'Sep',
            'Oct', 'Nov', 'Dec'
        ];

        let context = (await (msg.channel.messages.fetch({after: msg.id, limit: 50}))).array().reverse();

        context.unshift(msg);

        let groups = group_messages(context);

        let replies_to: string = await replies_at(msg);
        
        let image = '';
        let tweet_text = '';
        for (let msg of groups[0]) {
            let tmp_image = get_attachment(msg);
            let tmp_tweet_text = this.clean_content(msg.content, channel);
            if (!image && tmp_image) {
                image = tmp_image;
                if (image === tmp_tweet_text) {
                    tmp_tweet_text = '';
                }
            } 
            tweet_text += `${tmp_tweet_text}${tmp_tweet_text.length ? '\n' : ''}`;
        }
        tweet_text = tweet_text.replace(/\s+$/g, '');

        tweet_text = clean_content(tweet_text);
        tweet_text = emojify(tweet_text);

        let msg_time = new Date(msg.createdTimestamp);

        let retweets = rb_(this.text.tweetEsotericAmountBefore, '', 0.2) || 
                        number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
        let quotes = rb_(this.text.tweetEsotericAmountBefore, '', 0.2) || 
                        number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
        let likes = rb_(this.text.tweetEsotericAmountBefore, '', 0.2) || 
                        number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
        let any_numbers: boolean = retweets.length > 0 || quotes.length > 0 || likes.length > 0;

        let verified = author_member !== null && // Member exists and 
                        (!server.no_context_role ||   // Either this server has no context role or 
                          author_member.roles.cache.has(server.no_context_role.id)); // This member has no context role

        let tweet: TweetData = {
            theme: rb_(this.text.tweetTheme, 'dim', 2) as TweetTheme,
            retweeter: rb_(this.text.tweetRetweeter, retweeter.user.username, 0.5),
            avatar: author.displayAvatarURL(),
            name: author_member?.displayName ?? author.username,
            verified: verified,
            at: author.tag,
            replyTo: replies_to,
            tweetText: tweet_text,
            image: image,
            factCheck: rb_(this.text.tweetFactCheck, ''),
            hour: `${msg_time.getHours().toString().padStart(2, '0')}:${msg_time.getMinutes().toString().padStart(2, '0')}`,
            day: `${msg_time.getDate()}`,
            month: rb_(this.text.tweetMonth, `${months[msg_time.getMonth()]}`),
            year: `${msg_time.getFullYear()}`,
            client: rb_(this.text.tweetClient, 'Twitter Web App'),
            any_numbers: any_numbers,
            retweets: retweets,
            quotes: quotes,
            likes: likes,
            moreTweets: []
        };

        groups.shift(); // Remove first group

        if (add_extras) {

            for (let group of groups) {
                let extra = group[0]; // First msg
                const author = extra.author;
                const author_member = await guild.members.fetch(author.id);

                let time_str = '';
                const time_diff = new Date().getTime() - extra.createdTimestamp;
                if (time_diff < 60 * 1000) {
                    time_str = `${Math.floor(time_diff / 1000) % 60}s`;
                } else if (time_diff < 60 * 60 * 1000) {
                    time_str = `${Math.floor(time_diff / 1000 / 60) % 60}m`;
                } else if (time_diff < 24 * 60 * 60 * 1000) {
                    time_str = `${Math.floor(time_diff / 1000 / 60 / 60) % 24}h`;
                } else {
                    const time = new Date(extra.createdTimestamp);
                    time_str = `${time.getDate()} ${months[time.getMonth()]} ${time.getFullYear()}`;
                }

                let replies_to: string = await replies_at(extra);

                let image = '';
                let tweet_text = '';
                for (let msg of group) {
                    let tmp_image = get_attachment(msg);
                    let tmp_tweet_text = this.clean_content(msg.content, channel);
                    if (!image && tmp_image) {
                        image = tmp_image;
                        if (image === tmp_tweet_text) {
                            tmp_tweet_text = '';
                        }
                    } 
                    tweet_text += `${tmp_tweet_text}${tmp_tweet_text.length ? '\n' : ''}`;
                }
                tweet_text = tweet_text.replace(/\s+$/g, '');

                tweet_text = clean_content(tweet_text);
                tweet_text = emojify(tweet_text);

                let replies = rb_(this.text.tweetEsotericAmount, '', 0.05) || 
                                number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
                let retweets = rb_(this.text.tweetEsotericAmount, '', 0.05) || 
                            number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
                let likes = rb_(this.text.tweetEsotericAmount, '', 0.05) || 
                            number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));

                let verified = author_member !== null && // Member exists and 
                    (!server.no_context_role ||   // Either this server has no context role or 
                        author_member.roles.cache.has(server.no_context_role.id)); // This member has no context role

                let extra_tweet: TweetMoreData = {
                    avatar: author.displayAvatarURL(),
                    name: rb_(this.text.tweetUsername, author_member?.displayName ?? author.username, 0.2),
                    verified: verified,
                    at: author.tag,
                    time: rb_(this.text.tweetEsotericTime, time_str, 0.2),
                    replyTo: replies_to ? rb_(this.text.tweetExtraReply, replies_to, 0.2) : replies_to,
                    tweetText: rb_(this.text.tweetExtraText, tweet_text, 0.1),
                    image: image,
                    replies: replies,
                    retweets: retweets,
                    likes: likes
                };

                tweet.moreTweets.push(extra_tweet);
            }
        }

        let img = await createImage(tweet);

        await msg.channel.send({
            reply: {
                messageReference: msg
            },
            files: [{
                name: 'tweet.png', // TODO Funky funny hella names
                attachment: img 
            }]
        });
    }

    /** Pins a message to the hall of fame channel of a server 
     * 
     * If `forced` is true the forced pin emoji is used  
     */
    async pin(msg: D.Message, to: D.TextChannel, emoji: Emoji) {
        const server = this.get_server(msg);
        if (!server || server._beta !== this.beta || !this.can_talk(to)) {
            return false;
        }
        const pinchannel = to;
        if (!pinchannel || !pinchannel.isText()) {
            Logger.error(`Attempted to pin ${msg.id} in channel ${server.hof_channel}`);
            return;
        }

        function emoji_image(e: Emoji): string {
            const img = twemoji.parse(e.toString());
            const url = /src=\"(.*?)\"/.exec(img)?.[1];
            return url ?? 'https://twemoji.maxcdn.com/v/latest/72x72/2049.png';
        }

        const icon = emoji.id ? 
        `https://cdn.discordapp.com/emojis/${emoji.id}.${emoji.animated ? 'gif' : 'png'}` :
        emoji_image(emoji); 
        const r = Math.floor(Math.random() * 0x10) * 0x10;
        const g = Math.floor(Math.random() * 0x10) * 0x10;
        const b = Math.floor(Math.random() * 0x10) * 0x10;
        const embed: D.MessageEmbedOptions = {
            color: r << 16 | g << 8 | b, // Randomized color :D
            author: {
                name: `${msg.author.username}`,
                iconURL: msg.author.displayAvatarURL({format: 'png', size: 128})
            },
            // thumbnail: {
            //     url: msg.author.dynamicAvatarURL("png", 128)
            // },
            description: `${msg.content}`,
            timestamp: msg.createdTimestamp,
            footer: {
                text: `${msg.id} - ${msg.channel.id}`,
                iconURL: icon
            }
        };
        const guild_id = server.id;
        const channel_id = msg.channel.id;
        const message_id = msg.id;
        const url = `https://canary.discordapp.com/channels/${guild_id}/${channel_id}/${message_id}`;
        let desc = `[Click to teleport](${url})`;
        if(msg.attachments?.size){
            const attachment = msg.attachments.array()[0];
            const embedtype: 'video' | 'image' = /\.(webm|mp4)$/g.test(attachment.name ?? '') ? 'video' : 'image';
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
        pinchannel.send({ embeds: [embed] });
        return true;
    }

    /** Attempts to steal a puzzle clue */
    async maybe_steal(msg: D.Message, user: D.User) {
        // TODO Change stealing with fancy buttons
        if (!msg.reactions.resolve(emojis.devil.toString())?.me || this.message_locked(msg)) {
            return;
        }

        this.lock_message(msg);
        const content = msg.content;
        await msg.reactions.resolve(emojis.devil.toString())?.users.remove(this.client.user!.id);
        await msg.edit(`${rb_(this.text.puzzleSteal, 'Stolen')} by ${user.username}`);

        (await user.createDM()).send(content);
        this.db.addClueSteal(msg, user);
        this.add_cleanup_task(() => msg.delete(), 1000 * 5 * 60);
    }

    /** Attempts to add a message to the no-context channel */
    async maybe_remove_context(msg: D.Message) {
        const server = this.get_server(msg);
        if (!msg.guild || !server || server._beta !== this.beta || !server.no_context_channel) {
            return false;
        }
        const channel = server.no_context_channel;

        if (!this.can_talk(channel)) {
            return false;
        }

        // TODO Variable msgcontent length and chance?
        if (msg.content && msg.content.length <= 280) {
            // Post the message to the no-context channel
            channel.send({
                content: msg.content,
                files: msg.attachments.array()
            });
            this.transferXp(xp.secondsOfXp(60 * 60), null, msg.author, xpTransferReason.NoContext);
            if (server.no_context_role) {
                let role = server.no_context_role;
                // Shuffle the no-context role
                for (let [_, member] of msg.guild.members.cache) {
                    if (member.id === msg.author.id) {
                        member.roles.add(role);
                    } else if (member.roles.cache.has(role.id)) {
                        member.roles.remove(role);
                    }
                }
                randFromFile('nocontext.txt', 'No context', function(name) {
                    role.edit({name: name});
                });

                // TODO If variable chance, variable chance here too?
                if (server._puzzle_channel && Math.random() * 4 < 1.0) {
                    this.postClue(server._puzzle_channel.id);
                }
            }
        }
    }

    /** Connects the client */
    async connect() {
        this.loadText();
        this.client.login(this.token);
    }

    /** Disconnects the client and cleans up */
    async die() {
        try {
            if (this.cleanup_interval) {
                clearInterval(this.cleanup_interval); // Removes cleanup_interval TODO Is this correct?
            }

            await this.save(); 

            await this.run_cleanup(true);

            // Reset nickname
            const self = this;
            await Promise.all(this.client.guilds.cache.map((guild, guild_id) => {
                const server = self.servers[guild_id];
                if (!server || this.beta !== server._beta) {
                    return null;
                }

                if (server.nickname) {
                    return guild.me?.setNickname(server.nickname);
                } else {
                    return guild.me?.setNickname(null);
                }
            }));

        } catch (e) {
            Logger.error('Error while quitting: ', e.stack);
        } finally {
            // Do not reconnect
            this.client.destroy();
            this.db.close();
        }
    }
}
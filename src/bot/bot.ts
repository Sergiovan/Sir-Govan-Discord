import Eris from 'eris';
import * as util from 'util';

import { encode } from 'html-entities';
import * as twemoji from 'twemoji';

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
import { createImage, TweetData, TweetMoreData, TweetTheme } from '../twitter';

/** Pair containing command name and command function */
type Command = [string, CommandFunc];

const storage_location = 'data/node-persist';
const db_location = 'data/db/bot.db';

/** Pair containing a function and when to execute it */
type CleanupCode = [Date, () => void];

/** Wrapper over Eris Client class */
export class Bot {
    client: Eris.Client; // Eris client 
    owner?: Eris.User; // Me, Sergiovan. How exciting
    storage: Persist; // Basic persistent storage
    puzzler: Puzzler = new Puzzler; // Puzzle stuff
    db: DBWrapper; // Database for advanced persistent storage

    users: {[key: string]: BotUser} = {}; // A map from User ID to internal user representation

    commands: Command[] = []; // A list of all the commands available
    beta: boolean; // If the bot is running in beta mode
    
    cleanup_interval?: NodeJS.Timeout; // Interval for cleanup functions
    cleanup_list: CleanupCode[] = []; // List of to-be-cleaned-up actions
    message_mutex: Set<string> = new Set(); // Message lock
    
    text: { [key: string]: RarityBag } = {}; // Text instance, for random chance texts

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
        this.cleanup_interval = setInterval(() => this.run_cleanup(), 1000 * 60); // Cleanup runs every minute

        // Initialize storage and then load it too
        this.storage = new Persist(storage_location);
        this.storage.init().then(() => {
            this.load().catch(function(e) {
                console.log('Could not load data from file: ', e);
            }).finally(async function() { 
                self.startClues(); // After it's loaded we start the puzzle
            });
        });
    }

    /** Only called in the constructor, these are set just once */
    setFixedListeners() {
        for (let event in fixed_listeners) {
            this.client.removeAllListeners(event);
            this.client.on(event, fixed_listeners[event].bind(this));
        }
    }

    /** Called every time the bot restarts */
    setListeners() {
        for (let event in listeners) {
            this.client.removeAllListeners(event);
            this.client.on(event, listeners[event].bind(this));
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
        ]);
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
                    console.log('npm double SIGINT bug?: ', e);
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
        const seen: Set<string> = new Set<string>(); // Users we have already seen

        for (let user of db_users) {
            seen.add(user.id);
            if (this.client.users.has(user.id)) { // The bot can see this user
                user.is_member = 1;

                new_users[user.id] = new BotUser(user);

                user.commit();
            } else { // The bot cannot see this user anymore, ignore it
                user.is_member = 0;
                user.commit();
            }
        }

        // For each member in every server
        for (let [guild_id, guild] of this.client.guilds) {
            const server = botparams.servers.ids[guild_id.toString()];

            if (server && server.beta === this.beta) {
                for (let [member_id, member] of guild.members) {
                    if (seen.has(member_id.toString())) { // We've seen this user when going through the db
                        const db_user = new_users[member_id.toString()];
                        db_user.update_member(member); // Update its member data as well

                        db_user.commit();
                    } else {
                        // This user was not in the db, so add it now
                        const db_user = await this.db.addUser(member.user, 1, member.bot ? 1 : 0 , member.nick);
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

    /** Check if a certain message is locked 
     * 
     * Notice, this method of locking/unlocking only works because 
     * this app is single-threaded...
    */
    message_locked(msg: Eris.Message) {
        return this.message_mutex.has(msg.id);
    }
    
    /** Lock a message, unlock after a minute */
    lock_message(msg: Eris.Message, ms: number = 1000 * 60) {
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
            console.log('Forced task through');
            task();
        }
    }

    /** Parses a message to run a command */
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

    /** Adds a command to the list of commands */
    addCommand(name: string, command: CommandFunc) {
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
            console.error(e);
            console.log('Could not reload text');
            return [false, e];
        };
    }

    /** Begin the puzzle */
    async startClues() {
        const text = `${this.beta ? 'Beta message! ' : ''}${this.puzzler.startClues()}`;
        console.log(text);
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
            return; // Can happen if it's not time yet
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

    /** Verifies that `answer` is the answer to the current puzzle */
    async checkAnswer(answer: string, user: Eris.User) {
        // I cannot accidentally ruin the puzzle... Haha oops
        if (!this.owner || (user.id === this.owner.id && !this.beta)) {
            return;
        }

        // If the answer is correct
        if (this.puzzler.checkAnswer(answer)) {
            const id = this.puzzler.puzzle_id;
            this.puzzler.endPuzzle();
            const dm = await user.getDMChannel();
            dm.createMessage(rb_(this.text.answerCorrect, 'You got it!'));

            await this.tellTheBoss(`${user.username} (${user.id}) got it!`);

            // Start a new puzzle in an hour
            setTimeout(this.startClues.bind(this), 1000 * 60 * 60 * 24);

            const puzzle = await this.db.getPuzzle(id);
            if (puzzle && this.users[user.id]) {
                puzzle.winner = this.users[user.id].db_user.rowid;
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
    transferXp(amount: number, from: Eris.User, to: Eris.User, reason: xpTransferReason): boolean;
    transferXp(amount: number, from: Eris.User | null, to: Eris.User, reason: xpTransferReason): true;
    transferXp(amount: number, from: Eris.User, to: Eris.User | null, reason: xpTransferReason): boolean;
    transferXp(amount: number, from: Eris.User | null, to: Eris.User | null, reason: xpTransferReason) {
        if (from) {
            if (!this.users[from.id].remove_xp(amount)) {
                return false; // If we cannot remove this amount of xp from the donor, the whole thing is cancelled
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

    /** Transfer passive xp to a user when they talk */
    async tickUser(user: Eris.User) {
        const bot_user = this.users[user.id];
        if ((bot_user.last_spoke + xp.passive_timeout * 1000) < Date.now()) {
            bot_user.last_spoke = Date.now();
            this.transferXp(xp.secondsOfXp(xp.passive_timeout), null, user, xpTransferReason.Passive);
        }
    }

    /** Returns a cleaned string from a message content. Spaces in names are replaced with \0
     * and need to be returned to ' '
    */
    clean_content(msg: Eris.Message, channel: Eris.TextChannel): string {
        let text = msg.content;
        let self = this;

        text = text.replace(/<@!?([0-9]+)>/g, function(match: string, m1: string) {
            const m = channel.guild.members.get(m1);
            return `@${m?.nick ?? self.client.users.get(m1)?.username ?? "unknown-user"}`.replace(/ /g, '\x00');
        });

        text = text.replace(/<@\&([0-9]+)>/g, function(match: string, m1: string) {
            const r = channel.guild.roles.get(m1);
            return `#${r?.name ?? "deleted-role"}`.replace(/ /g, '\x00');
        });

        text = text.replace(/<#([0-9]+)>/g, function(match: string, m1: string) {
            const c = channel.guild.channels.get(m1);
            return `@${c?.name ?? "deleted-channel"}`.replace(/ /g, '\x00');
        });

        return text;
    }

    /** Writes a message on the same channel as `msg`
     * 
     * NOTE: Does not use inline replies!! 
     */
    reply(msg: Eris.Message, def: string, rb?: RarityBag) {
        return this.client.createMessage(msg.channel.id, rb_(rb, def));    
    }

    /** Writes a DM to the author of `msg` */
    async replyDM(msg: Eris.Message, def: string, rb?: RarityBag) {
        const channel = await msg.author.getDMChannel();
        return await this.client.createMessage(channel.id, rb_(rb, def));
    }

    /** Writes me, Sergiovan, a DM */
    async tellTheBoss(what: string) {
        console.log(`${'[BOSS]'.cyan} ${what}`);
        const ch = this.owner?.getDMChannel();
        return (await ch)?.createMessage(what);
    }

    /** Attempts to pin a message */
    async maybe_pin(msg: Eris.Message, emoji: Emoji) {
        const server = botparams.servers.getServer(msg);
        if (!server || server.beta !== this.beta || !server.pin_channel) {
            return;
        }

        const findname = emoji.id ? `${emoji.name}:${emoji.id}` : emoji.name;
        if (msg.author.bot) { // Do not pin bot messages
            return;
        }

        if ((msg.reactions[emojis.pushpin.fullName] && 
            msg.reactions[emojis.pushpin.fullName].me) ||
            this.message_locked(msg)) {
            return; // If this messages has been pinned or is locked for pinning, cease
        }

        let reactionaries = await msg.getReaction(findname);
        // At least `server.pin_amount` pins that are not the author or bots
        if(reactionaries.filter((user) => user.id !== msg.author.id && !user.bot).length >= server.pin_amount){
            //We pin that shit!
            this.lock_message(msg);
            await msg.addReaction(emojis.pushpin.fullName);
            this.pin(msg);
        }
    }

    /** Attempts to retweet a message */
    async maybe_retweet(msg: Eris.Message, retweeter: Eris.Member, add_extras: boolean) {
        let self = this;
        
        function get_at(usr: Eris.User) {
            return `${usr.username}#${usr.discriminator}`
        }

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
            text = text.replace(/&lt;\:.*?\:([0-9]+)&gt;/g, '<img class="emoji" src="https://cdn.discordapp.com/emojis/$1.png">');
            return text;
        }

        const server = botparams.servers.getServer(msg);
        if (!server || server.beta !== this.beta || !server.pin_channel) {
            return;
        }

        if (!(msg.channel instanceof Eris.TextChannel)) return; // No DMs

        const emoji = add_extras ? emojis.repeat.fullName : emojis.repeat_one.fullName;

        if ((msg.reactions[emoji] && msg.reactions[emoji].me) || this.message_locked(msg)) {
            return; // If this messages has been pinned or is locked for pinning, cease
        }

        this.lock_message(msg);
        await msg.addReaction(emoji);
        this.add_cleanup_task(() => {
            msg.removeReaction(emoji);
        }, 1000 * 60 * 30);

        const channel = msg.channel;
        const guild = channel.guild;

        const author = msg.author;
        const author_member = guild.members.get(author.id);

        const months = [
            'Jan', 'Feb', 'Mar',
            'Apr', 'May', 'Jun',
            'Jul', 'Aug', 'Sep',
            'Oct', 'Nov', 'Dec'
        ];

        let replies_to: string = '';
        
        if (msg.messageReference !== null) { // Reply
            if (msg.referencedMessage !== null) { // Message was not deleted
                let reply_msg: Eris.Message;
                if (msg.referencedMessage) {
                    reply_msg = msg.referencedMessage;
                } else {
                    reply_msg = await this.client.getMessage(msg.messageReference.channelID, msg.messageReference.channelID);
                }
                replies_to = get_at(reply_msg.author);
            }
        }

        
        let image = '';
        if (msg.attachments.length) {
            for (let att of msg.attachments) {
                if (!/\.(webm|mp4)$/g.test(att.filename)) { // Img
                    image = att.url;
                    break;
                }
            }
        }
        if (!image && msg.embeds.length) {
            for (let embed of msg.embeds) {
                if (embed.type === 'image') {
                    image = embed.thumbnail?.url ?? '';
                    break;
                }
            }
        }

        let tweet_text = this.clean_content(msg, channel);
        if (tweet_text === image) {
            tweet_text = '';
        }
        tweet_text = clean_content(tweet_text);
        tweet_text = emojify(tweet_text);

        let msg_time = new Date(msg.timestamp);

        let retweets = rb_(this.text.tweetEsotericAmount, '', 0.2) || 
                        number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
        let quotes = rb_(this.text.tweetEsotericAmount, '', 0.2) || 
                        number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
        let likes = rb_(this.text.tweetEsotericAmount, '', 0.2) || 
                        number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
        let any_numbers: boolean = retweets.length > 0 || quotes.length > 0 || likes.length > 0;

        let tweet: TweetData = {
            theme: rb_(this.text.tweetTheme, 'dim') as TweetTheme,
            retweeter: rb_(this.text.tweetRetweeter, retweeter.username),
            avatar: author.avatarURL,
            name: author_member?.nick ?? author.username,
            verified: !!author_member,
            at: get_at(author),
            replyTo: replies_to,
            tweetText: tweet_text,
            image: image,
            factCheck: rb_(this.text.tweetFactCheck, ''),
            hour: `${msg_time.getHours().toString().padStart(2, '0')}:${msg_time.getMinutes().toString().padStart(2, '0')}`,
            day: `${msg_time.getDate()}`,
            month: rb_(this.text.tweetMonth, `${months[msg_time.getMonth()]}`),
            year: `${msg_time.getFullYear()}`,
            client: rb_(this.text.twitterClient, 'Twitter Web App'),
            any_numbers: any_numbers,
            retweets: retweets,
            quotes: quotes,
            likes: likes,
            moreTweets: []
        };

        if (add_extras) {
            let extras = await this.client.getMessages(msg.channel.id, {
                after: msg.id,
                limit: 10
            });

            for (let extra of extras.reverse()) {
                const author = extra.author;
                const author_member = guild.members.get(author.id);

                let time_str = '';
                const time_diff = new Date().getTime() - extra.timestamp;
                if (time_diff < 60 * 1000) {
                    time_str = `${Math.floor(time_diff / 1000) % 60}s`;
                } else if (time_diff < 60 * 60 * 1000) {
                    time_str = `${Math.floor(time_diff / 1000 / 60) % 60}m`;
                } else if (time_diff < 24 * 60 * 60 * 1000) {
                    time_str = `${Math.floor(time_diff / 1000 / 60 / 60) % 24}h`;
                } else {
                    const time = new Date(extra.timestamp);
                    time_str = `${time.getDate()} ${months[time.getMonth()]} ${time.getFullYear()}`;
                }

                let replies_to: string = '';
            
                if (extra.messageReference !== null) { // Reply
                    if (extra.referencedMessage !== null) { // Message was not deleted
                        let reply_msg: Eris.Message;
                        if (extra.referencedMessage) {
                            reply_msg = extra.referencedMessage;
                        } else {
                            reply_msg = await this.client.getMessage(extra.messageReference.channelID, extra.messageReference.channelID);
                        }
                        replies_to = get_at(reply_msg.author);
                    }
                }

                let image = '';
                if (extra.attachments.length) {
                    for (let att of extra.attachments) {
                        if (!/\.(webm|mp4)$/g.test(att.filename)) { // Img
                            image = att.url;
                            break;
                        }
                    }
                }
                if (!image && extra.embeds.length) {
                    for (let embed of extra.embeds) {
                        if (embed.type === 'image') {
                            image = embed.thumbnail?.url ?? '';
                            break;
                        }
                    }
                }

                let tweet_text = this.clean_content(extra, channel);
                if (tweet_text === image) {
                    tweet_text = '';
                }
                tweet_text = clean_content(tweet_text);
                tweet_text = emojify(tweet_text);

                let replies = rb_(this.text.tweetEsotericAmount, '', 0.05) || 
                                number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
                let retweets = rb_(this.text.tweetEsotericAmount, '', 0.05) || 
                            number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));
                let likes = rb_(this.text.tweetEsotericAmount, '', 0.05) || 
                            number_to_twitter_text(random_tweet_number(), rb_(this.text.tweetAmountSymbol, '', 0.2));

                let extra_tweet: TweetMoreData = {
                    avatar: author.avatarURL,
                    name: rb_(this.text.tweetUsername, author_member?.nick ?? author.username, 0.2),
                    verified: !!author_member,
                    at: get_at(author),
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

        await msg.channel.createMessage({
            messageReference: {
                channelID: msg.channel.id,
                messageID: msg.id
            }
        }, {
            name: 'tweet.png',
            file: img 
        });
    }

    /** Pins a message to the hall of fame channel of a server 
     * 
     * If `forced` is true the forced pin emoji is used  
     */
    pin(msg: Eris.Message, forced: boolean = false) {
        const server = botparams.servers.getServer(msg);
        if (!server || server.beta !== this.beta || !server.pin_channel) {
            return false;
        }
        const pinchannel = server.pin_channel;
        const icon = forced ? 
            'https://emojipedia-us.s3.amazonaws.com/thumbs/120/twitter/131/double-exclamation-mark_203c.png' : 
            'https://cdn.discordapp.com/emojis/263774481233870848.png';
        const r = Math.floor(Math.random() * 0x10) * 0x10;
        const g = Math.floor(Math.random() * 0x10) * 0x10;
        const b = Math.floor(Math.random() * 0x10) * 0x10;
        const embed: Eris.Embed = {
            type: 'rich',
            color: r << 16 | g << 8 | b, // Randomized color :D
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
        const guild_id = server.id;
        const channel_id = msg.channel.id;
        const message_id = msg.id;
        const url = `https://canary.discordapp.com/channels/${guild_id}/${channel_id}/${message_id}`;
        let desc = `[Click to teleport](${url})`;
        if(msg.attachments && msg.attachments.length){
            const attachment = msg.attachments[0];
            const embedtype: 'video' | 'image' = /\.(webm|mp4)$/g.test(attachment.filename) ? 'video' : 'image';
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

    /** Attempts to steal a puzzle clue */
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

    /** Attempts to add a message to the no-context channel */
    async tryRemoveContext(msg: Eris.Message, server: Server) {
        const channel = server.no_context_channel;

        // TODO Variable msgcontent length and chance?
        if (channel && msg.cleanContent && msg.cleanContent.length <= 280 && !msg.attachments.length) {
            // Post the message to the no-context channel
            this.client.createMessage(channel, msg.cleanContent);
            this.transferXp(xp.secondsOfXp(60 * 60), null, msg.author, xpTransferReason.NoContext);
            if (server.no_context_role) {
                // Shuffle the no-context role
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

                // TODO If variable chance, variable chance here too?
                if (server.puzzle_channel && Math.random() * 4 < 1.0) {
                    this.postClue(server.puzzle_channel);
                }
            }
        }
    }

    /** Connects the client */
    async connect() {
        this.loadText();
        this.client.connect();
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
            // Do not reconnect
            this.client.disconnect({reconnect: false});
            this.db.close();
        }
    }
}
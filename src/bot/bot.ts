import * as D from 'discord.js';
import * as util from 'util';

import { encode } from 'html-entities';
import twemoji from 'twemoji';

import { CommandFunc } from './commands';
import { cmds, aliases, beta_cmds } from './commands';
import { listeners, fixed_listeners, ListenerFunction } from './listeners';

import { emojis, Emoji, JsonableServer, Server, xpTransferReason } from '../defines';
import { randFromFile, RarityBag, rb_, Logger, Mutex } from '../utils';

import { Persist } from '../data/persist';

import { xp } from '../secrets/secrets';
import { createImage, TweetData, TweetMoreData, TweetTheme } from './twitter';
import { join } from 'path';
import { make_titlecard } from './titlecard';
import { Screenshotter } from './screenshots';

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

    commands: Command[] = []; // A list of all the commands available
    beta: boolean; // If the bot is running in beta mode
    
    cleanup_interval?: NodeJS.Timeout; // Interval for cleanup functions
    cleanup_list: CleanupCode[] = []; // List of to-be-cleaned-up actions

    channel_mutexes: {[key: D.Snowflake]: Mutex } = {}; // Mutexes for reacting and such. Only one per channel

    randomize_timeout: NodeJS.Timeout | null = null; // Interval for randomizing

    text: { [key: string]: RarityBag } = {}; // Text instance, for random chance texts

    servers: { [key: string]: Server } = {};
    ownerID: D.Snowflake = '120881455663415296'; // Sergiovan#0831
    #server_loaded: boolean = false;

    constructor(token: string, beta: boolean) {
        this.token = token;

        const Flags = D.GatewayIntentBits;
        const Partials = D.Partials;
        this.client = new D.Client({
            intents: [
                Flags.Guilds,
                Flags.GuildMembers,
                Flags.GuildVoiceStates,
                Flags.GuildPresences,
                Flags.GuildMessages,
                Flags.GuildMessageReactions,
                Flags.DirectMessages,
                Flags.DirectMessageReactions,
                Flags.MessageContent,
            ],
            partials: [
                Partials.Message,
                Partials.Channel,
                Partials.Reaction
            ]
        });

        this.beta = beta;

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
        return;
    }

    /** Saves all basic permanent storage */
    async save() {
        return Promise.all([
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
        return Promise.resolve(true);
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
            if (on.channel.type === D.ChannelType.DM) {
                return true;
            } else if (on.channel.type === D.ChannelType.GuildNews) {
                return false;
            }
            channel = on.channel;
        } else if (!on.isTextBased()) { 
            return false;
        } else {
            channel = on;
        }
        if (channel instanceof D.ThreadChannel) {
            if (!channel.parent || channel.parent.type === D.ChannelType.GuildNews) {
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
        const perms = channel.permissionsFor(channel.guild.members.me!);
        const f = D.PermissionFlagsBits;
        return perms.has(f.ViewChannel | f.SendMessages) && server.allowed_commands(channel);
    }

    // Bot can send messages and listen
    can_talk(on: D.Message | D.GuildChannel | D.ThreadChannel): boolean {
        let channel = this.#channelize(on);
        if (channel === true || channel === false) return channel;

        const server = this.get_server(on);
        if (!server) return false;
        const perms = channel.permissionsFor(channel.guild.members.me!);
        const f = D.PermissionFlagsBits;
        return perms.has(f.ViewChannel | f.SendMessages);
    }

    // Bot can listen
    can_listen(on: D.Message | D.GuildChannel | D.ThreadChannel): boolean {
        let channel = this.#channelize(on);
        if (channel === true || channel === false) return channel;

        const server = this.get_server(on);
        if (!server) return false;
        const perms = channel.permissionsFor(channel.guild.members.me!);
        const f = D.PermissionFlagsBits;
        return perms.has(f.ViewChannel | f.ReadMessageHistory) && server.allowed_listen(channel);
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

    /**
     * Performs a task (runs a function) with an async-mutex and returns the result. Each channel
     * has its own mutex, as cross-channel activities don't require mutexes (yet)
     */
    async locked_task<T>(ch: D.Channel, task: ((() => T) | (() => PromiseLike<T>))) {
        // It's fine to do this because this will always be synchronous
        if (!this.channel_mutexes[ch.id]) {
            this.channel_mutexes[ch.id] = new Mutex();
        }
        const mut = this.channel_mutexes[ch.id];

        return await mut.dispatch(task);
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

    /** Returns a cleaned string from a message content. Spaces in names are replaced with \0
     * and need to be returned to ' '
    */
    clean_content(msg: D.Message | string, channel: D.TextChannel | D.ThreadChannel | D.VoiceChannel): string {
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
            if (!guild.members.me) {
                continue;
            }
            Logger.debug(`Setting nickname to "${new_nick}" in ${guild.name}`);
            promises.push(guild.members.me.setNickname(new_nick));
        }

        // LISTENING: Listening to
        // WATCHING: Watching
        // PLAYING: Playing
        // STREAMING: Playing (But the profile says "Live on xxx")
        // COMPETING: Competing in
        const doing = rb_(this.text.status_type, '') as 'Listening' | 'Watching' | 'Playing' | 'Streaming' | 'Competing' | '';
        if (doing !== '') {
            let activity = D.ActivityType[doing];
            const texts = {
                Listening: this.text.status_listening,
                Watching: this.text.status_watching,
                Playing: this.text.status_playing,
                Streaming: this.text.status_watching,
                Competing: this.text.status_competing
            };

            this.client.user!.setActivity(rb_(texts[doing], 'something'), {type: activity});
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

    get_member_role(member: D.GuildMember) {
        let guild = member.guild;

        let user_roles = member.roles.cache.filter(role => !!role.color);
        user_roles.sort((a, b) => b.position - a.position);

        // Edit the first unique role
        for (let [role_id, role] of user_roles) {
            if (guild.members.cache.find((usr) => usr.id !== member.id && usr.roles.cache.has(role_id))) {
                continue;
            }
            return role;
        }

        return null;
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

        const res = await this.locked_task(msg.channel as D.Channel, async() => {
            msg = await msg.fetch();
            
            // Do not pin messages where I've already reacted
            const recs = msg.reactions.resolve(emoji.to_reaction_resolvable());
            if (!recs || recs.me) {
                return false; // If this messages has been pinned or is locked for pinning, cease
            }

            let reactionaries = await recs.users.fetch();
            if (!reactionaries) return false; // ???

            if (reactionaries.filter((user) => user.id !== msg.author.id && !user.bot).size >= server.pin_amount) {
                await msg.react(emoji.toString());
                return true;
            } else {
                return false;
            }
        });

        if (!res) return;
            
        await this.pin(msg, to, pinmoji);
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
                callback: function(icon: string, options: TwemojiOptions) {
                    switch ( icon ) {
                        case 'a9':      // © copyright
                        case 'ae':      // ® registered trademark
                        case '2122':    // ™ trademark
                            return false;
                    }
                    return `${options.base}${options.size}/${icon}${options.ext}`;
                }
            }) as any as string; // TODO Remove once twemoji 14.0.1 is gone
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
        if (!server || !server.hof_channel) {
            return;
        }

        if (msg.channel.type === D.ChannelType.DM || msg.channel.type === D.ChannelType.GuildNews) return; // No DMs or... news... channels?

        const emoji = add_extras ? emojis.repeat : emojis.repeat_one;

        let res = await this.locked_task(msg.channel, async () => {
            msg = await msg.fetch();
            let recs = msg.reactions.resolve(emoji.to_reaction_resolvable());
            
            if (recs?.me) {
                return false; // If this messages has been pinned or is locked for pinning, cease
            }
            
            await msg.react(emoji.toString());
            return true;
        });

        // No res means the reaction was already there 
        if (!res) return;

        // From this point we're safe, as long as we call only_once
        this.add_cleanup_task(async () => {
            const reactions = await msg.reactions.resolve(emoji.to_reaction_resolvable());
            if (!reactions) return;
            await reactions.users.remove(self.client.user!.id);
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

        let msgs_coll = await msg.channel.messages.fetch({after: msg.id, limit: 50});

        let context: D.Message<boolean>[] = Array.from(msgs_coll.values()).reverse(); // TODO does this alaways work?

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

    async maybe_titlecard(msg: D.Message, creator: D.GuildMember) {
        // TODO Put these in utils

        function clean_content(text: string) {
            text = encode(text);

            text = text.replace(/\n/g, '<br>');
            text = text.replace(/\x00/g, ' ');

            return text;
        }

        function emojify(text: string) {
            text = twemoji.parse(text, {
                callback: function(icon: string, options: TwemojiOptions) {
                    switch ( icon ) {
                        case 'a9':      // © copyright
                        case 'ae':      // ® registered trademark
                        case '2122':    // ™ trademark
                            return false;
                    }
                    return `${options.base}${options.size}/${icon}${options.ext}`;
                }
            }) as any as string; // TODO Remove once twemoji 14.0.1 is gone
            text = text.replace(/&lt;a?\:.*?\:([0-9]+)&gt;/g, '<img class="emoji" src="https://cdn.discordapp.com/emojis/$1.png">');
            return text;
        }
        
        function fix_name(text: string) {
            text = text.replace(/-/g, ' ');
            text = text.charAt(0).toUpperCase() + text.slice(1);

            text = emojify(text);
            return text;
        }

        const server = this.get_server(msg);
        if (!server || !msg.content.length || msg.embeds.length || msg.attachments.size) {
            return; // No empty messages or messages with embeds or attachments
        }

        if (msg.channel.type === D.ChannelType.DM || msg.channel.type === D.ChannelType.GuildNews) return; // No DMs or... news... channels?

        const emoji = server.titlecard_emoji;
        if (!emoji) {
            return; // Cannot happen but makes ts happy
        }

        let reactions: D.MessageReaction;
        const res = await this.locked_task(msg.channel, async () => {
            msg = await msg.fetch(); // Update message
            const recs = msg.reactions.resolve(emoji.to_reaction_resolvable());

            if (!recs || recs?.me) {
                return false; // If this messages has been pinned or is locked for pinning, cease
            }

            reactions = recs;
            await msg.react(emoji.toString());
            return true;
        });

        if (!res || !reactions!) return;

        let episode_title = this.clean_content(msg.content, msg.channel);
        episode_title = episode_title.replace(/\s+$/g, '');
        episode_title = clean_content(episode_title);
        episode_title = emojify(episode_title);
        episode_title = `"${episode_title}"`;

        const song_file = join('media', 'tempsens.ogg');
        let show_name: string;
        if (Math.random() < 0.1) { 
            show_name = rb_(this.text.titlecardShowEntire, "It's Always Sunny in Philadelphia");
        } else {
            show_name = rb_(this.text.titlecardShowPrefix, "It's Always Sunny in");
            show_name += " ";
            show_name += fix_name(Math.random() < 0.2 ? msg.channel.name : msg.channel.guild.name);
        }

        const vid = await make_titlecard(episode_title, show_name, song_file); // TODO Funky filenames

        await msg.channel.send({
            content: undefined,
            files: [{
                attachment: vid,
                name: 'iasip.mp4' // TODO Funky stuff yee
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
        if (!pinchannel || !pinchannel.isTextBased()) {
            Logger.error(`Attempted to pin ${msg.id} in channel ${server.hof_channel}`);
            return false;
        }

        function emoji_image(e: Emoji): string {
            const img = twemoji.parse(e.toString()) as any as string; // TODO Remove once twemoji 14.0.1 is gone
            const url = /src=\"(.*?)\"/.exec(img)?.[1];
            return url ?? 'https://twemoji.maxcdn.com/v/latest/72x72/2049.png';
        }

        const icon = emoji.id ? 
        `https://cdn.discordapp.com/emojis/${emoji.id}.${emoji.animated ? 'gif' : 'png'}` :
        emoji_image(emoji); 
        const r = Math.floor(Math.random() * 0x10) * 0x10;
        const g = Math.floor(Math.random() * 0x10) * 0x10;
        const b = Math.floor(Math.random() * 0x10) * 0x10;
        const embed = new D.EmbedBuilder()
            .setColor(r << 16 | g << 8 | b)
            .setAuthor({
                name: `${msg.author.username}`,
                iconURL: msg.author.displayAvatarURL({size: 128})
            })
            .setDescription(msg.content || null)
            .setTimestamp(msg.createdTimestamp)
            .setFooter({
                text: `${msg.id} - ${msg.channel.id}`,
                iconURL: icon
            });
        const url = msg.url;
        let desc = `[Click to teleport](${url})`;
        if (msg.attachments?.size) {
            const attachment = Array.from(msg.attachments.values())[0];
            const embedtype: 'video' | 'image' = /\.(webm|mp4)$/g.test(attachment.name ?? '') ? 'video' : 'image';
            switch (embedtype) {
                case 'video':
                    break;
                case 'image':
                    embed.setImage(attachment.url)
                    break;
            }
            
            if (embedtype === 'video') {
                desc = `[Click to go to video](${url})`;
            }
        } else if (msg.embeds && msg.embeds.length) {
            let nembed = msg.embeds[0];
            if (nembed.video) {
                desc = `[Click to go to video](${url})`;
            }
            if (nembed.image) { 
                embed.setImage(nembed.image.proxyURL || '');
            }
        } else if (msg.stickers.size) {
            embed.setImage(msg.stickers.first()!.url);
        }
        if(!msg.content) {
            embed.setDescription(desc);
        } else {
            embed.addFields([{
                "name": "\u200b",
                "value": desc
            }]);
        }
        pinchannel.send({ embeds: [embed] });
        return true;
    }

    /** Attempts to steal a puzzle clue */
    async maybe_steal(msg: D.Message, user: D.User) {
        if (!msg.reactions.resolve(emojis.devil.toString())?.me) {
            return;
        }

        const content = msg.content;
        await msg.reactions.resolve(emojis.devil.toString())?.users.remove(this.client.user!.id);
        await msg.edit(`${rb_(this.text.puzzleSteal, 'Stolen')} by ${user.username}`);

        (await user.createDM()).send(content);
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
                files: Array.from(msg.attachments.values())
            });
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
            }
            return true;
        }

        return false;
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

            await this.run_cleanup(true);

            if (this.randomize_timeout) {
                clearTimeout(this.randomize_timeout);
            }
            
            await this.save(); 
            
            // Reset nickname
            const self = this;
            await Promise.all(this.client.guilds.cache.map((guild, guild_id) => {
                const server = self.servers[guild_id];
                if (!server || this.beta !== server._beta) {
                    return null;
                }

                if (server.nickname) {
                    return guild.members.me?.setNickname(server.nickname);
                } else {
                    return guild.members.me?.setNickname(null);
                }
            }));

        } catch (e: any) {
            Logger.error('Error while quitting: ', e.stack);
        } finally {
            // Do not reconnect
            this.client.destroy();
            await Screenshotter.deinit();
        }
    }
}
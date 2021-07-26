import * as D from 'discord.js';
import * as util from 'util';

export interface JsonableServer {
    id: D.Snowflake;
    _beta: boolean;
    nickname: string;

    allowed_channels_commands: Array<D.Snowflake>;
    disallowed_channels_listen: Array<D.Snowflake>;
    
    hof_channel: D.Snowflake | null;
    hof_amount: number;
    hof_emoji: JsonableEmoji;
    
    no_context_channel: D.Snowflake | null;
    no_context_role: D.Snowflake | null;

    _puzzle_channel: D.Snowflake | null;
}

type ServerHelper = {[T in keyof JsonableServer]: any};

export class Server implements ServerHelper {
    id: D.Snowflake;
    _beta: boolean;
    nickname: string;

    allowed_channels_commands: Set<D.Snowflake>;
    disallowed_channels_listen: Set<D.Snowflake>;
    
    hof_channel: D.TextChannel | null;
    hof_amount: number;
    hof_emoji: Emoji;
    
    no_context_channel: D.TextChannel | null;
    no_context_role: D.Role | null;

    _puzzle_channel: D.TextChannel | null;

    constructor(client: D.Client, id: D.Snowflake, jsonable: Partial<JsonableServer>) {
        this.id = id;
        const guild = client.guilds.cache.get(id);
        this._beta = jsonable._beta || false;
        this.nickname = jsonable.nickname || '';

        this.allowed_channels_commands = new Set(jsonable.allowed_channels_commands || []);
        this.disallowed_channels_listen = new Set(jsonable.disallowed_channels_listen || []);
        
        this.hof_channel = (jsonable.hof_channel && guild?.channels.cache.get(jsonable.hof_channel) as D.TextChannel) ?? null;
        this.hof_amount = jsonable.hof_amount || 3; // Default pin amount is 3
        this.hof_emoji = jsonable.hof_emoji ? new Emoji(jsonable.hof_emoji) : emojis.pushpin;
        
        this.no_context_channel = (jsonable.no_context_channel && guild?.channels.cache.get(jsonable.no_context_channel) as D.TextChannel) ?? null;
        this.no_context_role = (jsonable.no_context_role && guild?.roles.cache.get(jsonable.no_context_role)) ?? null;

        this._puzzle_channel = (jsonable._puzzle_channel && guild?.channels.cache.get(jsonable._puzzle_channel) as D.TextChannel) ?? null;
    }

    as_jsonable(): JsonableServer {
        return {
            id: this.id,
            _beta: this._beta,
            nickname: this.nickname,

            allowed_channels_commands: Array.from(this.allowed_channels_commands),
            disallowed_channels_listen: Array.from(this.disallowed_channels_listen),

            hof_channel: this.hof_channel?.id ?? null,
            hof_amount: this.hof_amount,
            hof_emoji: this.hof_emoji,

            no_context_channel: this.no_context_channel?.id ?? null,
            no_context_role: this.no_context_role?.id ?? null,

            _puzzle_channel: this._puzzle_channel?.id ?? null
        }
    }

    [util.inspect.custom](depth: number, opts: any) {
        return this.as_jsonable();
    }

    /**
     * If the channel can be sent a command to
     */
    allowed_commands(channel: D.Channel): boolean {
        return this.allowed_channels_commands.has(channel.id);
    }

    /**
     * If the bot can listen to events on this channel
     */
    allowed_listen(channel: D.Channel): boolean {
        return !this.disallowed_channels_listen.has(channel.id);
    }
}

interface JsonableEmoji {
    name: string;
    id: string | null;
    animated: boolean;
}

export class Emoji implements JsonableEmoji {
    name: string;
    id: string | null;
    animated: boolean;

    constructor({name, id = null, animated = false}: {name: string, id?: string | null, animated?: boolean}) {
        this.name     = name;
        this.id       = id;
        this.animated = animated;
    }

    toString(): string {
        if (this.id) {
            return `<${this.animated ? 'a' : ''}:${this.name}:${this.id}>`
        } else {
            return this.name;
        }
    }

    to_reaction_resolvable(): string {
        return this.id ?? this.name;
    }
}

// getServer: (msg: Eris.Message) => Server | undefined

// type BotParams = {
//     servers: {
//         ids: {
//             [key: string]: Server
//         },
//         getServer: (msg: D.Message) => Server | undefined
//     },
//     owner: D.Snowflake
// };

// export const botparams: BotParams = {
//     servers: {
//         ids: {
//             '120581475912384513': new Server('120581475912384513', { // The comfort zone
//                 beta: true,
//                 allowed_channels: [
//                     '216992217988857857',  // #807_73571n6
//                 ],  
//                 allowed_channels_listen: [
//                     '120581475912384513',  // #meme-hell
//                     '216992217988857857' // #807_73571n6
//                 ],
//                 pin_channel: '216992217988857857', // #807_73571n6
//                 no_context_channel: '422797217456324609', // #no-context
//                 no_context_role: '424933828826497024',
//                 puzzle_channel: '216992217988857857', // #807_73571n6
//             }),
//             '140942235670675456': new Server('140942235670675456', { // The club
//                 beta: false,
//                 nickname: 'Admin bot',
//                 allowed_channels: [
//                     '271748090090749952',   // #config-chat
//                     '222466032290234368'	// #bot-chat
//                 ],  
//                 allowed_channels_listen: [
//                     '140942235670675456',  // #main-chat 
//                     '415173193133981718'   // #drawing-discussion
//                 ],
//                 pin_channel: '422796631235362841',  // #hall-of-fame
//                 no_context_channel: '422796552935964682',  // #no-context
//                 no_context_role: '424949624348868608',
//                 puzzle_channel: '271748090090749952', // #config-chat
//             }),
//             '785872130029256743': new Server('785872130029256743', { // mmk server
//                 beta: false,
//                 nickname: "Sosa's husband",
//                 allowed_channels: [
//                     // Empty, only listen
//                 ],
//                 allowed_channels_listen: [
//                     '785872439812816936', // talky talky
//                     '785873031494369311', // bee tee es
//                     '785894960243146752', // anym
//                     '814794847277023232', // tunes
//                     '785873644525584394', // simp
//                     '785872130029256747', // welcum // Note: I did not come up with these names, ok?
//                     '824345444649140224', // it's called art baby look it up
//                     '831961057194016778', // cum is a meal replacement
//                     '847251232796311552', // gaymers playing genshin
//                     '838014715519041556', // tinder men suck
//                     '858006470238142474', // astro sluts
//                     '835566308782768199', // dee pee ar
//                 ],
//                 pin_channel: '822930237418504312',
//             })
//         },
//         getServer(msg: D.Message): Server | undefined {
//             if(!msg.guild) {
//                 return undefined;
//             }
//             return this.ids[msg.guild.id];
//         }
//     },
//     owner: '120881455663415296' // Sergiovan#0831
// };

export const emojis = {
    pushpin: new Emoji({name: '📌'}),
    reddit_gold: new Emoji({name: 'redditgold', id: '263774481233870848'}),
    ok_hand: new Emoji({name: '👌'}),
    fist: new Emoji({name: '✊'}),
    exlamations: new Emoji({name: '‼️'}),
    devil: new Emoji({name: '😈'}),
    repeat: new Emoji({name: '🔁'}),
    repeat_one: new Emoji({name: '🔂'}),
};

export enum argType {
    string = 0,
    number = 1,
    user = 2,
    channel = 3,
    role = 4,
    bigint = 5,
    boolean = 6,
    emoji = 7,
    rest = 100
};

export enum xpTransferReason {
    Passive,
    NoContext,

};

export const regexes = {
    emoji: /(\u00a9|\u00ae|[\u2000-\u3300]|\ud83c[\ud000-\udfff]|\ud83d[\ud000-\udfff]|\ud83e[\ud000-\udfff])/,
    discord_emojis: /^(?:\`|\\)?\<(a?)\:(.*?)\:([0-9]+)\>\`?$/,
    discord_user: /(?:<@!?)?([0-9]+)>?/,
    discord_channel: /(?:<#)?([0-9]+)>?/,
    discord_role: /(?:<@\&)?([0-9]+)>?/
};
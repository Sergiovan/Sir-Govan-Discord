import * as D from 'discord.js';
import * as util from 'util';

export interface JsonableServer {
    id: D.Snowflake;
    _beta: boolean;
    nickname: string;

    allowed_channels_commands: Array<D.Snowflake>;
    disallowed_channels_listen: Array<D.Snowflake>;
    
    pin_amount: number;
    
    hof_channel: D.Snowflake | null;
    hof_emoji: JsonableEmoji;
    
    vague_channel: D.Snowflake | null;
    vague_emoji: JsonableEmoji;

    word_wrong_channel: D.Snowflake | null;
    word_wrong_emoji: JsonableEmoji;

    anything_pin_channel: D.Snowflake | null;

    no_context_channel: D.Snowflake | null;
    no_context_role: D.Snowflake | null;

    _puzzle_channel: D.Snowflake | null;

    titlecard_emoji: JsonableEmoji | null;
}

type ServerHelper = {[T in keyof JsonableServer]: any};

export class Server implements ServerHelper {
    id: D.Snowflake;
    _beta: boolean;
    nickname: string;

    allowed_channels_commands: Set<D.Snowflake>;
    disallowed_channels_listen: Set<D.Snowflake>;
    
    pin_amount: number;

    hof_channel: D.TextChannel | null;
    hof_emoji: Emoji;
    
    vague_channel: D.TextChannel | null;
    vague_emoji: Emoji;

    word_wrong_channel: D.TextChannel | null;
    word_wrong_emoji: Emoji;

    anything_pin_channel: D.TextChannel | null;

    no_context_channel: D.TextChannel | null;
    no_context_role: D.Role | null;

    _puzzle_channel: D.TextChannel | null;

    titlecard_emoji: Emoji | null;

    constructor(client: D.Client, id: D.Snowflake, jsonable: Partial<JsonableServer>) {
        this.id = id;
        const guild = client.guilds.cache.get(id);
        this._beta = jsonable._beta || false;
        this.nickname = jsonable.nickname || '';

        this.allowed_channels_commands = new Set(jsonable.allowed_channels_commands || []);
        this.disallowed_channels_listen = new Set(jsonable.disallowed_channels_listen || []);
        
        this.pin_amount = jsonable.pin_amount || 3; // Default pin amount is 3

        this.hof_channel = (typeof jsonable.hof_channel === 'string' && guild?.channels.cache.get(jsonable.hof_channel) as D.TextChannel) || null;
        this.hof_emoji = jsonable.hof_emoji ? new Emoji(jsonable.hof_emoji) : emojis.pushpin;
        
        this.vague_channel = (typeof jsonable.vague_channel === 'string' && guild?.channels.cache.get(jsonable.vague_channel) as D.TextChannel) || null;
        this.vague_emoji = jsonable.vague_emoji ? new Emoji(jsonable.vague_emoji) : emojis.no_mouth;

        this.word_wrong_channel = (typeof jsonable.word_wrong_channel === 'string' && guild?.channels.cache.get(jsonable.word_wrong_channel) as D.TextChannel) || null;
        this.word_wrong_emoji = jsonable.word_wrong_emoji ? new Emoji(jsonable.word_wrong_emoji) : emojis.weary;

        this.anything_pin_channel = (typeof jsonable.anything_pin_channel === 'string' && guild?.channels.cache.get(jsonable.anything_pin_channel) as D.TextChannel) || null;

        this.no_context_channel = (typeof jsonable.no_context_channel === 'string' && guild?.channels.cache.get(jsonable.no_context_channel) as D.TextChannel) || null;
        this.no_context_role = (typeof jsonable.no_context_role === 'string' && guild?.roles.cache.get(jsonable.no_context_role)) || null;

        this._puzzle_channel = (typeof jsonable._puzzle_channel === 'string' && guild?.channels.cache.get(jsonable._puzzle_channel) as D.TextChannel) || null;
    
        this.titlecard_emoji = jsonable.titlecard_emoji ? new Emoji(jsonable.titlecard_emoji) : null;
    }

    as_jsonable(): JsonableServer {
        return {
            id: this.id,
            _beta: this._beta,
            nickname: this.nickname,

            allowed_channels_commands: Array.from(this.allowed_channels_commands),
            disallowed_channels_listen: Array.from(this.disallowed_channels_listen),

            pin_amount: this.pin_amount,

            hof_channel: this.hof_channel?.id ?? null,
            hof_emoji: this.hof_emoji,

            vague_channel: this.vague_channel?.id ?? null,
            vague_emoji: this.vague_emoji,

            word_wrong_channel: this.word_wrong_channel?.id ?? null,
            word_wrong_emoji: this.word_wrong_emoji,

            anything_pin_channel: this.anything_pin_channel?.id ?? null,

            no_context_channel: this.no_context_channel?.id ?? null,
            no_context_role: this.no_context_role?.id ?? null,

            _puzzle_channel: this._puzzle_channel?.id ?? null,

            titlecard_emoji: this.titlecard_emoji ?? null
        }
    }

    [util.inspect.custom](depth: number, opts: any) {
        return this.as_jsonable();
    }

    /**
     * If the channel can be sent a command to
     */
    allowed_commands(channel: D.GuildChannel): boolean {
        return this.allowed_channels_commands.has(channel.id);
    }

    /**
     * If the bot can listen to events on this channel
     */
    allowed_listen(channel: D.GuildChannel): boolean {
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

export const emojis = {
    pushpin: new Emoji({name: '📌'}),
    reddit_gold: new Emoji({name: 'redditgold', id: '263774481233870848'}),
    ok_hand: new Emoji({name: '👌'}),
    fist: new Emoji({name: '✊'}),
    exlamations: new Emoji({name: '‼️'}),
    devil: new Emoji({name: '😈'}),
    repeat: new Emoji({name: '🔁'}),
    repeat_one: new Emoji({name: '🔂'}),
    japanese_ogre: new Emoji({name: '👹'}),
    weary: new Emoji({name: '😩'}),
    no_mouth: new Emoji({name: '😶'}),
    violin: new Emoji({name: '🎻'}),
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
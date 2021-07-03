import * as D from 'discord.js';

// TODO This has to go
export class Server {
    id: D.Snowflake;
    beta: boolean;
    nickname: string;

    allowed_channels: Array<D.Snowflake>;
    allowed_channels_listen: Array<D.Snowflake>;
    
    pin_channel: D.Snowflake | null;
    pin_amount: number;
    
    no_context_channel: D.Snowflake | null;
    no_context_role: D.Snowflake | null;

    puzzle_channel: D.Snowflake | null;

    constructor(id: D.Snowflake, obj: Partial<Server>) {
        this.id = id;
        this.beta = obj.beta || false;
        this.nickname = obj.nickname || '';

        this.allowed_channels = obj.allowed_channels || [];
        this.allowed_channels_listen = obj.allowed_channels_listen || [];
        
        this.pin_channel = obj.pin_channel || null;
        this.pin_amount = obj.pin_amount || 3; // Default pin amount is 3
        
        this.no_context_channel = obj.no_context_channel || null;
        this.no_context_role = obj.no_context_role || null;

        this.puzzle_channel = obj.puzzle_channel || null;
    }

    allowed(msg: D.Message): boolean {
        return this.allowed_channels.includes(msg.channel.id);
    }

    allowedListen(msg: D.Message): boolean {
        return this.allowed_channels_listen.includes(msg.channel.id);
    }
}

export class Emoji {
    name: string;
    id: string | null;
    animated: boolean;

    constructor(name: string, id: string | null = null, animated: boolean = false) {
        this.name     = name;
        this.id       = id;
        this.animated = animated;
    }

    get asReaction(): string {
        if (this.id) {
            return `${this.id}`;
        } else {
            return this.name;
        }
    }

    get asContent(): string {
        if (this.id) {
            return `${this.animated ? 'a:' : ''}${this.name}:${this.id}`
        } else {
            return this.name;
        }
    }
}

// getServer: (msg: Eris.Message) => Server | undefined

type BotParams = {
    servers: {
        ids: {
            [key: string]: Server
        },
        getServer: (msg: D.Message) => Server | undefined
    },
    owner: D.Snowflake
};

export const botparams: BotParams = {
    servers: {
        ids: {
            '120581475912384513': new Server('120581475912384513', { // The comfort zone
                beta: true,
                allowed_channels: [
                    '216992217988857857',  // #807_73571n6
                ],  
                allowed_channels_listen: [
                    '120581475912384513',  // #meme-hell
                    '216992217988857857' // #807_73571n6
                ],
                pin_channel: '216992217988857857', // #807_73571n6
                no_context_channel: '422797217456324609', // #no-context
                no_context_role: '424933828826497024',
                puzzle_channel: '216992217988857857', // #807_73571n6
            }),
            '140942235670675456': new Server('140942235670675456', { // The club
                beta: false,
                nickname: 'Admin bot',
                allowed_channels: [
                    '271748090090749952',   // #config-chat
                    '222466032290234368'	// #bot-chat
                ],  
                allowed_channels_listen: [
                    '140942235670675456',  // #main-chat 
                    '415173193133981718'   // #drawing-discussion
                ],
                pin_channel: '422796631235362841',  // #hall-of-fame
                no_context_channel: '422796552935964682',  // #no-context
                no_context_role: '424949624348868608',
                puzzle_channel: '271748090090749952', // #config-chat
            }),
            '785872130029256743': new Server('785872130029256743', { // mmk server
                beta: false,
                nickname: "Sosa's husband",
                allowed_channels: [
                    // Empty, only listen
                ],
                allowed_channels_listen: [
                    '785872439812816936', // talky talky
                    '785873031494369311', // bee tee es
                    '785894960243146752', // anym
                    '814794847277023232', // tunes
                    '785873644525584394', // simp
                    '785872130029256747', // welcum // Note: I did not come up with these names, ok?
                    '824345444649140224', // it's called art baby look it up
                    '831961057194016778', // cum is a meal replacement
                    '847251232796311552', // gaymers playing genshin
                    '838014715519041556', // tinder men suck
                    '858006470238142474', // astro sluts
                    '835566308782768199', // dee pee ar
                ],
                pin_channel: '822930237418504312',
            })
        },
        getServer(msg: D.Message): Server | undefined {
            if(!msg.guild) {
                return undefined;
            }
            return this.ids[msg.guild.id];
        }
    },
    owner: '120881455663415296' // Sergiovan#0831
};

export const emojis = {
    pushpin: new Emoji('📌'),
    reddit_gold: new Emoji('redditgold', '263774481233870848'),
    ok_hand: new Emoji('👌'),
    fist: new Emoji('✊'),
    exlamations: new Emoji('‼️'),
    devil: new Emoji('😈'),
    repeat: new Emoji('🔁'),
    repeat_one: new Emoji('🔂'),
};

export enum argType {
    string = 0,
    number = 1,
    user = 2,
    channel = 3,
    role = 4,
    bigint = 5,
    rest = 100
};

export enum xpTransferReason {
    Passive,
    NoContext,

};
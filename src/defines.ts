"use strict";

import Eris from 'eris';

type ServerObjParam = {
    beta?: boolean;
    allowed_channels?: Array<string>;
    allowed_channels_listen?: Array<string>;
    pin_channel?: string;
    no_context_channel?: string;
    no_context_role?: string;
    nickname?: string;
};

class Server {
    id: string;
    beta: boolean;
    allowed_channels: Array<string>;
    allowed_channels_listen: Array<string>;
    pin_channel: string;
    no_context_channel: string;
    no_context_role: string;
    nickname: string;

    constructor(id: string, obj: ServerObjParam) {
        this.id = id;
        this.beta = obj.beta || false;
        this.allowed_channels = obj.allowed_channels || [];
        this.allowed_channels_listen = obj.allowed_channels_listen || [];
        this.pin_channel = obj.pin_channel || '';
        this.no_context_channel = obj.no_context_channel || '';
        this.no_context_role = obj.no_context_role || '';
        this.nickname = obj.nickname || '';
    }

    allowed(msg: Eris.Message): boolean {
        if (!msg || !msg.channel) {
            return false;
        }
        return this.allowed_channels.includes(msg.channel.id);
    }

    allowedListen(msg: Eris.Message): boolean {
        if (!msg || !msg.channel) {
            return false;
        }
        return this.allowed_channels_listen.includes(msg.channel.id);
    }
}

class Emoji {
    name: string;
    id: string | null;
    animated: boolean;

    constructor(name: string, id: string | null = null, animated: boolean = false) {
        this.name     = name;
        this.id       = id;
        this.animated = animated;
    }

    get fullName(): string {
        if (this.id) {
            return `${this.name}:${this.id}`;
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
        getServer: (msg: Eris.Message) => Server | undefined
    },
    owner: string
};

const botparams: BotParams = {
    servers: {
        ids: {
            '120581475912384513': new Server('120581475912384513', { // The comfort zone
                beta: true,
                allowed_channels: [
                    '216992217988857857',  // #807_73571n6
                ],  
                allowed_channels_listen: [
                    '120581475912384513',  // #meme-hell
                ],
                pin_channel: '216992217988857857', // #807_73571n6
                no_context_channel: '422797217456324609', // #no-context
                no_context_role: '424933828826497024',
            }),
            '140942235670675456': new Server('140942235670675456', { // The club
                beta: false,
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
                nickname: 'Admin bot'
            }),
        },
        getServer(msg: Eris.Message): Server | undefined {
            if(!msg?.channel?.guild) {
                return undefined;
            }
            return this.ids[(msg.channel as Eris.TextChannel).guild.id];
        }
    },
    owner: '120881455663415296' // Sergiovan#0831
};

const emojis = {
    pushpin: new Emoji('📌'),
    reddit_gold: new Emoji('redditgold', '263774481233870848'),
    ok_hand: new Emoji('👌'),
    fist: new Emoji('✊'),
    exlamations: new Emoji('‼️')
};

enum argTypes {
    string = 0,
    number = 1,
    user = 2,
    channel = 3,
    role = 4,
    rest = 100
};

export { botparams, emojis, argTypes };
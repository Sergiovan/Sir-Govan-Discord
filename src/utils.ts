import { Guild, GuildMember, User, TextChannel, Role, Message, Snowflake } from 'discord.js';

import 'colors';

import * as util from 'util';
import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';

import { argType, Emoji, regexes } from './defines';
import { performance } from 'perf_hooks';

// Yoinked from https://github.com/Morglod/ts-tuple-hacks/blob/master/index.ts
// Nabs the length of a tuple as a literal type
export type TupleLen<T extends unknown[]> = T['length'];
// Grabs the head of a tuple
export type TupleHead<T extends unknown[]> = T extends [infer U, ...any[]] ? U : never;
// Grabs the tail of a tuple. For 1-element tuples it returns `never` instead of `[]`
export type TupleTail<T extends unknown[]> = T extends [unknown, ...infer U] ? (U extends [] ? never : U) : never;

export type Return<T extends Function> = T extends (...x: any[]) => infer U ? U : never;

export type CallResult<T extends Function, Args extends any[]> = T extends (...x: Args) => infer U ? U : never;

interface StringArg {
    type: argType.string;
    value: string | null;
    nullable: boolean;
}

interface NumberArg {
    type: argType.number;
    value: number | null;
    nullable: boolean;
}

interface UserArg {
    type: argType.user;
    value: string | null;
    nullable: boolean;
}

interface ChannelArg {
    type: argType.channel;
    value: string | null;
    nullable: boolean;
}

interface RoleArg {
    type: argType.role;
    value: string | null;
    nullable: boolean;
}

interface BigIntArg {
    type: argType.bigint;
    value: bigint | null;
    nullable: boolean;
}

interface BooleanArg {
    type: argType.boolean;
    value: boolean | null;
    nullable: boolean;
}

interface EmojiArg {
    type: argType.emoji,
    value: Emoji | null,
    nullable: boolean
}

interface RestArg {
    type: argType.rest;
    value: string | null;
    nullable: boolean;
}

export type Arg = StringArg | NumberArg | UserArg | ChannelArg | 
                  RoleArg | BigIntArg | BooleanArg | EmojiArg | RestArg;

// Decodes argument type to the object it's encoding
type DeArg<T extends Arg> = 
    (T extends StringArg ? string :
    T extends NumberArg ? number :
    T extends UserArg ? GuildMember : 
    T extends ChannelArg ? TextChannel :
    T extends RoleArg ? Role : 
    T extends BigIntArg ? bigint :
    T extends BooleanArg ? boolean : 
    T extends EmojiArg ? Emoji :
    T extends RestArg ? string : never) | null | undefined;

// Ho boy. Decodes all arguments in an array, given:
type DeArgsHelper<H extends Arg, T> = 
    T extends never ? [DeArg<H>] : // If there is no tail, simply return an array containing this very element
    T extends Arg[] ?  // If there is a tail, however
                        (T['length'] extends 1 ? [DeArg<H>, DeArg<T[0]>] : // For a length 1 tail, just finish up the thing
                         [DeArg<H>, ...DeArgsHelper<T[0], TupleTail<T>>]) : // Otherwise recursively call this with the tail
    [never]; // Delete the planet

// Decodes all arguments in an array
type DeArgs<T extends Arg[]> = 
    TupleLen<T> extends 1 ? 
    [DeArg<T[0]>] : // Single argument, do simple decoding
    DeArgsHelper<T[0], TupleTail<T>>; // Multiple arguments, go mad

export function arg (type: argType.string, value?: string | null, nullable?: boolean): StringArg;
export function arg (type: argType.number, value?: number | null, nullable?: boolean): NumberArg;
export function arg (type: argType.user, value?: string | null, nullable?: boolean): UserArg;
export function arg (type: argType.channel, value?: string | null, nullable?: boolean): ChannelArg;
export function arg (type: argType.role, value?: string | null, nullable?: boolean): RoleArg;
export function arg (type: argType.bigint, value?: bigint | null, nullable?: boolean): BigIntArg;
export function arg (type: argType.boolean, value? : boolean | null, nullable?: boolean): BooleanArg;
export function arg (type: argType.emoji, value? : Emoji | null, nullable?: boolean): EmojiArg;
export function arg (type: argType.rest, value?: string | null, nullable?: boolean): RestArg;

export function arg(type: argType, value: any = null, nullable: boolean = false): Arg {
    return {
        type: type,
        value: value,
        nullable: nullable
    };
}

export function parseArgs<T extends Arg[]>(msg: Message, ...args: T): [...DeArgs<T>] {
    const first_space = msg.content.indexOf(' ');
    return parseArgsHelper(first_space === -1 ? '' : msg.content.slice(first_space+1).trimStart(), msg.guild, ...args);
}

// TODO Get a more powerful parser for Discord Commands
export function parseArgsHelper<T extends Arg[]>(text: string, guild: Guild | null, ...args: T): [...DeArgs<T>] {
    type Return = [...DeArgs<T>];

    const ret: [...any[]] = []; 
    const words = text.replace(/ +/g, ' ').split(' ');
    let word = 0; 
    // Labels... whack
    loop: for (let arg of args) {
        let inspected = words[word] || undefined;
        if (arg.nullable && inspected === 'null') {
            ret.push(arg.value ?? null);
            word++;
            continue;
        }
        let def = arg.value ?? undefined;
        switch (arg.type) {
            case argType.string:
                if (inspected || def) {
                    ret.push(inspected || def)
                } else {
                    break loop;
                }
                break;
            case argType.number:
                if (inspected && Number.isFinite(+inspected) && !Number.isNaN(+inspected)) {
                    ret.push(+inspected);
                } else {
                    if (arg.value) {
                        ret.push(arg.value);
                    } else {
                        break loop;
                    }
                }
                break;
            case argType.user: {
                const regex_res = regexes.discord_user.exec(inspected || arg.value || '');
                if (guild && regex_res && regex_res[1]) {
                    let member = guild.members.resolve(regex_res[1] as Snowflake);
                    if (member) {
                        ret.push(member);
                        break;
                    }
                }
                break loop;
            }
            case argType.channel: {
                const regex_res = regexes.discord_channel.exec(inspected || arg.value || '');
                if (guild && regex_res && regex_res[1]) {
                    let channel = guild.channels.resolve(regex_res[1] as Snowflake);
                    if (channel && channel.isTextBased()) {
                        ret.push(channel);
                        break;
                    }
                }
                break loop;
            }
            case argType.role: {
                const regex_res = regexes.discord_role.exec(inspected || arg.value || '');
                if (guild && regex_res && regex_res[1]) {
                    let role = guild.roles.resolve(regex_res[1] as Snowflake);
                    if (role) {
                        ret.push(role);
                        break;
                    }
                }
                break loop;
            }
            case argType.bigint: {
                let num: BigInt;
                try {
                    // For some fucking reason empty string parses to 0n????? Seriously???
                    num = BigInt(inspected || 'a');
                } catch (e) {
                    if (arg.value) {
                        num = arg.value;
                    } else {
                        break loop;
                    }
                }
                ret.push(num);
                break;
            }
            case argType.boolean: {
                switch (inspected) {
                    case 'true': ret.push(true); break;
                    case 'false': ret.push(false); break;
                    default: 
                        if (arg.value) {
                            ret.push(arg.value);
                        } else {
                            break loop;
                        }
                        break;
                }
                break;
            }
            case argType.emoji: {
                let regex_res = regexes.emoji.exec(inspected || '');
                if (regex_res && regex_res[1]) {
                    ret.push(new Emoji({name: regex_res[1]}));
                    break;
                }
                regex_res = regexes.discord_emojis.exec(inspected || '');
                if (regex_res && regex_res[2] && regex_res[3]) {
                    ret.push(new Emoji({name: regex_res[2], id: regex_res[3], animated: regex_res[1] !== ''}));
                    break;
                }
                if (arg.value) {
                    ret.push(arg.value);
                } else {
                    break loop;
                }
                break;
            }
            case argType.rest:
                ret.push(words.slice(word).join(' '));
                break;
            default:
                throw new Error('Undefined argument');
        }
        word++;
    }
    return ret as Return;
}

export async function sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export function randFromFile(filename: string, def: string, cb: (s: string) => void) {
    fs.readFile(path.join('data', filename), "utf8", function(err, data) {
        if(err) {
            Logger.error(`Error detected: ${err}`);
            cb(def);
        } else {
            const lines = data.trim().split('\n');
            cb(lines[Math.floor(Math.random() * lines.length)]);
        }
    });
}

export function randomBigInt(max: bigint = 20n, min: bigint = 0n): bigint {
    const range = 1n + max - min;
    const bytes = BigInt(Math.ceil(range.toString(2).length / 8));
    const bits = bytes * 8n;
    const buckets = (2n ** bits) / range;
    const limit = buckets * range;
    let r: bigint;

    do {
        r = BigInt('0x' + crypto.randomBytes(Number(bytes)).toString('hex'));
    } while (r >= limit);

    return min + (r / buckets);
}

/** Gotten from https://stackoverflow.com/a/55699349 */
export function randomEnum<T>(anEnum: T): T[keyof T] {
    const enumValues = Object.keys(anEnum)
        .map(n => Number.parseInt(n))
        .filter(n => !Number.isNaN(n)) as unknown as T[keyof T][];
    const randomIndex = Math.floor(Math.random() * enumValues.length);
    const randomEnumValue = enumValues[randomIndex];
    return randomEnumValue;
  }

/** Gotten from https://stackoverflow.com/a/2450976 */
export function shuffleArray<T>(arr: Array<T>) {
    let curr: number = arr.length;
    let tempVal: T;
    let randIndex: number;

    while (curr !== 0) {
        randIndex = Math.floor(Math.random() * curr);
        curr--;

        tempVal = arr[curr];
        arr[curr] = arr[randIndex];
        arr[randIndex] = tempVal;
    }
}

export function randomElement<T>(arr: Array<T>): T {
    return arr[Math.floor(Math.random() * arr.length)];
}

export function randomLetter(): string {
    const mask = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%&?'.split('');
    return randomElement(mask);
}

export function randomCode(): string {
    const len = 16; // 16, huh?
    const mask = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%&?'.split('');
    shuffleArray(mask);

    return mask.slice(0, len).join('');
}

export function fullName(user: User): string {
    return `${user.username}#${user.discriminator}`
}

export enum Rarity {
    Common = 0, // 75% - 100%
    Uncommon = 1, // 20% - 25%
    Rare = 2, // 4% - 5%
    Mythical = 3, // 0.99% - 1%
    WhatTheActualFuck = 4 // 0.01% - 0.01%
}

export type RBElementValue = string | (() => string);

export class RBElement {
    value: RBElementValue;
    rarity: Rarity;

    constructor(value: RBElementValue, rarity: Rarity) {
        this.value = value;
        this.rarity = rarity;
    }
} 

interface RarityBagConstructor {
    common?: Array<RBElementValue>;
    uncommon?: Array<RBElementValue>;
    rare?: Array<RBElementValue>;
    mythical?: Array<RBElementValue>;
    wtf?: Array<RBElementValue>;
}

export class RarityBag {   
    elements: Map<Rarity, RBElementValue[]> = new Map([
        [Rarity.Common, []],
        [Rarity.Uncommon, []],
        [Rarity.Rare, []],
        [Rarity.Mythical, []],
        [Rarity.WhatTheActualFuck, []]
    ]);

    constructor({
        common = [],
        uncommon = [],
        rare = [],
        mythical = [],
        wtf = []
    }: RarityBagConstructor) {
        this.elements.get(Rarity.Common)!.push(...common);
        this.elements.get(Rarity.Uncommon)!.push(...uncommon);
        this.elements.get(Rarity.Rare)!.push(...rare);
        this.elements.get(Rarity.Mythical)!.push(...mythical);
        this.elements.get(Rarity.WhatTheActualFuck)!.push(...wtf);
    }

    static unpack(val: RBElementValue): string {
        if (typeof val === 'string') {
            return val;
        } else {
            return val();
        }
    }

    static pickOrDefault(bag: RarityBag | undefined | null, def: RBElementValue, modifier: number = 1): string {
        let chosen: RBElementValue;
        if (bag) {
            chosen = bag.get(def, modifier);
        } else {
            chosen = def;
        }

        return RarityBag.unpack(chosen);
    }

    add(val: RBElement): void;
    add(val: RBElementValue, rarity: Rarity): void;
    add(val: RBElementValue[], rarity: Rarity): void;

    add(val: RBElement | RBElementValue | RBElementValue[], rarity?: Rarity) {
        if (val instanceof RBElement) {
            this.elements.get(val.rarity)?.push(val.value);
        } else if (Array.isArray(val)) {
            this.elements.get(rarity!)?.push(...val);
        } else {
            this.elements.get(rarity!)?.push(val);
        }
    }

    get(def: RBElementValue, modifier: number, rarity?: Rarity): RBElementValue {
        if (!rarity) {
            rarity = this.randomRarity(modifier);
        } 
        const elems = this.elements.get(rarity!);
        return elems?.length ? randomElement(elems) : def;
    } 

    randomRarity(modifier: number): Rarity {
        const rarities = [1 * modifier, 100 * modifier, 500 * modifier, 2500 * modifier];
        const rand = Number(randomBigInt(9999n)); // [0-9999], 10000 values
        if (rand < rarities[0] && this.elements.get(Rarity.WhatTheActualFuck)?.length) { // 1/10000
            return Rarity.WhatTheActualFuck;
        } else if (rand < rarities[1] && this.elements.get(Rarity.Mythical)?.length) { // 99/10000 
            return Rarity.Mythical;
        } else if (rand < rarities[2] && this.elements.get(Rarity.Rare)?.length) { // 400/10000
            return Rarity.Rare;
        } else if (rand < rarities[3] && this.elements.get(Rarity.Uncommon)?.length) { // 2000/10000
            return Rarity.Uncommon;
        } else { // 7500/10000
            return Rarity.Common;
        }
    }
}

export const rb_ = RarityBag.pickOrDefault;

/**
 * async-await mutex class taken from 
 * https://spin.atomicobject.com/2018/09/10/javascript-concurrency/ 
 */
export class Mutex {
    // Empty promise
    #mutex = Promise.resolve();

    /**
     * This is a really fucking complicated function, so I'll take some time to explain it.
     * 
     * NOTES:
     * The promise constructor takes 2 parameters, a resolve function, and a reject function.
     * 
     *  
     * First, we make a promise that does not resolve immediately. This is `ret`.
     * The resolve function of `ret` is stored in `begin`, because the promise constructor is called immediately.
     * This means calling `begin()` will resolve `ret`
     * Next, we chain `#mutex` to itself by making it return a new Promise.
     * This promise takes as input the resolve function of `ret`. 
     * Because of how promises are constructed, as soon as this constructor is called `ret` will resolve, and
     * it will have the resolve function of the `#mutex` chain promise as value.
     * That means `ret` will resolve to a function, which when called will resolve `#mutex`
     * 
     * THIS MEANS that the first call to `lock` will resolve immediately, since `#mutex` starts resolved, but
     * any subsequent calls need to wait until the previous call is over, which is notified by calling the 
     * return value of `lock`. Fucking hell
     */

    lock(): PromiseLike<() => void> {
        // A function taking a function returning void as a parameter, that returns void
        // Used to store resolver function of returned promise
        let begin: (unlock: () => void) => void = (unlock) => {};

        // Res takes a function returning void
        let ret: Promise<() => void> = new Promise((res, rej) => {
            begin = res; // This is done before the function passed to then() is called
            // Calling begin() will resolves #mutex
        });

        this.#mutex = this.#mutex.then(() => {
            // By the time this is called "begin" is the res function of the returned promise
            return new Promise(begin); // This fulfills ret and passes the fullfil function of itself
        });

        return ret;
    }

    async dispatch<T>(fn: (() => T) | (() => PromiseLike<T>)): Promise<T> {
        const unlock = await this.lock();
        
        try {
            return await Promise.resolve(fn());
        } finally {
            unlock();
        }
    }
}

export class Logger {
    static readonly INFO    = 'INFO   '.cyan;
    static readonly DEBUG   = 'DEBUG  '.green;
    static readonly WARNING = 'WARNING'.yellow;
    static readonly ERROR   = 'ERROR  '.red;
    static readonly INSPECT = 'INSPECT'.white;
    static readonly TIME    = 'TIME   '.magenta;
    static readonly padding_len = 14 + 2 + 7 + 1;
    static readonly padding = '\n' + ' '.repeat(Logger.padding_len);
    static #previous_day: number = -1;
    static #labels: {[key: string]: number} = {};

    static #time(): string {
        const now = new Date();
        if (this.#previous_day !== now.getDate()) {
            const day = `${now.getDate()}-${now.getMonth()+1}-${now.getFullYear()}`;
            const str = `~~~ ${day} ~~~`;
            this.#to_stdout(str);
            this.#to_stderr(str);
            this.#previous_day = now.getDate();
        }
        return `[${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}` +
               `:${now.getSeconds().toString().padStart(2, '0')}.${now.getMilliseconds().toString().padStart(3, '0')}]`;
    }

    static #stringify(args: any[]): string {
        return args.map(o => o && o['toString'] ? o.toString() as string : o as string).join(', ');
    }
    
    static #to_stdout(str: string): void {
        console.log(str);
    }

    static #to_stderr(str: string): void {
        console.error(str);
    }

    static info(...args: any[]): void {
        let str = this.#stringify(args);
        const prefix = `${this.#time()}[${this.INFO}] `;
        str = str.replace(/\n/g, this.padding);
        this.#to_stdout(prefix + str);
    }

    static debug(...args: any[]): void {
        let str = this.#stringify(args);
        const prefix = `${this.#time()}[${this.DEBUG}] `;
        str = str.replace(/\n/g, this.padding);
        this.#to_stdout(prefix + str);
    }

    static warning(...args: any): void {
        let str = this.#stringify(args);
        const prefix = `${this.#time()}[${this.WARNING}] `;
        str = str.replace(/\n/g, this.padding);
        this.#to_stdout(prefix + str);
    }

    static error(...args: any[]): void {
        let str = this.#stringify(args);
        const prefix = `${this.#time()}[${this.ERROR}] `;
        str = str.replace(/\n/g, this.padding);
        this.#to_stdout(prefix + str);
        this.#to_stderr(prefix + str);
    }

    static time_start(label: string): void {
        const now = performance.now();
        this.#labels[label] = now;
    }

    static time_get(label: string): number {
        const now = performance.now();
        return now - (this.#labels[label] ?? now);
    }

    static time_end(label: string): void {
        const now = performance.now();
        if (!(label in this.#labels)) {
            return;
        }
        const diff = now - this.#labels[label];
        
        const prefix = `${this.#time()}[${this.TIME}] `;
        this.#to_stdout(prefix + `${label}: ${diff.toFixed(3)}ms`);
        delete this.#labels[label];
    }

    static inspect(arg: any): void {
        let str = typeof arg === 'string' ? arg : util.inspect(arg, {colors: true, depth: 4});
        const prefix = `${this.#time()}[${this.INSPECT}] `;
        str = str.replace(/\n/g, this.padding);
        this.#to_stdout(prefix + str);
    }
}
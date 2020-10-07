"use strict";

import Eris from 'eris';

import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';

import {argType} from './defines';

interface StringArg {
    type: argType.string;
    value: string | null;
}

interface NumberArg {
    type: argType.number;
    value: number | null;
}

interface UserArg {
    type: argType.user;
    value: string | null;
}

interface ChannelArg {
    type: argType.channel;
    value: string | null;
}

interface RoleArg {
    type: argType.role;
    value: string | null;
}

interface RestArg {
    type: argType.rest;
    value: string | null;
}

type Arg = StringArg | NumberArg | UserArg | ChannelArg | RoleArg | RestArg;

export function arg (type: argType.string, value?: string | null): StringArg;
export function arg (type: argType.number, value?: number | null): NumberArg;
export function arg (type: argType.user, value?: string | null): UserArg;
export function arg (type: argType.channel, value?: string | null): ChannelArg;
export function arg (type: argType.role, value?: string | null): RoleArg;
export function arg (type: argType.rest, value?: string | null): RestArg;

export function arg(type: argType, value: any = null): Arg {
    return {
        type: type,
        value: value
    };
}

export function parseArgs(msg: Eris.Message, ...args: Arg[]) {
    let ret: Array<boolean | number | string | Eris.Member | Eris.Channel | Eris.Role | null | undefined> = [false];
    let word = 1;
    let words = msg.content.replace(/ +/g, ' ').split(' ');
    let specialHolder;
    for (let arg of args) {
        let inspected = words[word];
        switch (arg.type) {
            case argType.string:
                ret.push(inspected || arg.value);
                break;
            case argType.number:
                if (inspected && Number.isFinite(+inspected) && !Number.isNaN(+inspected)) {
                    ret.push(+inspected);
                } else {
                    if (arg.value) {
                        ret.push(arg.value);
                        word--;
                    } else {
                        return [true];
                    }
                }
                break;
            case argType.user:
                specialHolder = /<@!?([0-9]+?)>/.exec(inspected);
                if (specialHolder && specialHolder[1]) {
                    if ((msg.channel as Eris.TextChannel).guild.members.get(specialHolder[1])) {
                        ret.push((msg.channel as Eris.TextChannel).guild.members.get(specialHolder[1]));
                        break;
                    }
                }
                return [true];
            case argType.channel:
                specialHolder = /<#([0-9]+?)>/.exec(inspected);
                if (specialHolder && specialHolder[1]) {
                    if ((msg.channel as Eris.TextChannel).guild.channels.get(specialHolder[1])) {
                        ret.push((msg.channel as Eris.TextChannel).guild.channels.get(specialHolder[1]));
                        break;
                    }
                }
                return [true];
            case argType.role:
                specialHolder = /<@\&([0-9]+?)>/.exec(inspected);
                if (specialHolder && specialHolder[1]) {
                    if ((msg.channel as Eris.TextChannel).guild.roles.get(specialHolder[1])) {
                        ret.push((msg.channel as Eris.TextChannel).guild.roles.get(specialHolder[1]));
                        break;
                    }
                }
                return [true];
            case argType.rest:
                ret.push(words.slice(word).join(' '));
                break;
            default:
                throw new Error('Undefined argument');
        }
        word++;
    }
    return ret;
}

export function randFromFile(filename: string, def: string, cb: (s: string) => void) {
    fs.readFile(path.join('data', filename), "utf8", function(err, data) {
        if(err) {
            console.log(`Error detected: ${err}`);
            cb(def);
        } else {
            let lines = data.trim().split('\n');
            cb(lines[Math.floor(Math.random() * lines.length)]);
        }
    });
}

export function randomBigInt(max: bigint = 20n, min: bigint = 0n): bigint {
    let r: bigint;
    const range = 1n + max - min;
    const bytes = BigInt(Math.ceil(range.toString(2).length / 8));
    const bits = bytes * 8n;
    const buckets = (2n ** bits) / range;
    const limit = buckets * range;

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
    let mask = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%&?'.split('');
    shuffleArray(mask);

    return mask.slice(0, len).join('');
}

export enum Rarity {
    Common = 0, // 75%
    Uncommon = 1, // 20%
    Rare = 2, // 4%
    Mythical = 3, // 0.99%
    WhatTheActualFuck = 4 // 0.01%
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

    static pickOrDefault(bag: RarityBag | undefined | null, def: RBElementValue): string {
        let chosen: RBElementValue;
        if (bag) {
            chosen = bag.get(def);
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

    get(def: RBElementValue, rarity?: Rarity): RBElementValue {
        if (!rarity) {
            rarity = this.randomRarity();
        } 
        let elems = this.elements.get(rarity!);
        return elems?.length ? randomElement(elems) : def;
    } 

    randomRarity(): Rarity {
        let rand = randomBigInt(9999n); // [0-9999], 10000 values
        if (rand === 0n && this.elements.get(Rarity.WhatTheActualFuck)?.length) { // 1/10000
            return Rarity.WhatTheActualFuck;
        } else if (rand < 100n && this.elements.get(Rarity.Mythical)?.length) { // 99/10000 
            return Rarity.Mythical;
        } else if (rand < 500n && this.elements.get(Rarity.Rare)?.length) { // 400/10000
            return Rarity.Rare;
        } else if (rand < 2500n && this.elements.get(Rarity.Uncommon)?.length) { // 2000/10000
            return Rarity.Uncommon;
        } else { // 7500/10000
            return Rarity.Common;
        }
    }
}

export let rb_ = RarityBag.pickOrDefault;
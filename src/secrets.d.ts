"use strict";

/** Just fill in these functions */

export const discord = {
    token: "your-token-goes-here"
};

export enum ClueType { };

export class Clue {
    value: string;
    cycle_end: boolean = false;

    constructor(value: string, cycle_end: boolean = false) {
        this.value = value;
        this.cycle_end = cycle_end;
    }
};

export type ClueGenerator = Generator<string, void, unknown>;

export function * mysteryGenerator(answer: string, clue_type: ClueType): ClueGenerator {
    
}

export function clueHelp(clue_type: ClueType): string {
    return '';
}
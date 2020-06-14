"use strict";

/** Just fill in these functions */

export const discord = {
    token: "your-token-goes-here"
};

export enum ClueType { };
export type ClueGenerator = Generator<string, void, unknown>;

export function * mysteryGenerator(answer: string, clue_type: ClueType): ClueGenerator {
    
}

export function clueHelp(clue_type: ClueType): string {
    return '';
}
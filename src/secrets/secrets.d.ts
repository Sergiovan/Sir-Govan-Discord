/** Just fill in these functions */

export const discord: {
    token: string
};

export const xp: {
    secondsOfXp: (number) => number,
    levelToXp: (number) => number,
    XpToLevel: (number) => number,
    max_level: number,
};

export enum ClueType { }

export class Clue {
    value: string;
    cycle_end: boolean;

    constructor(value: string, cycle_end: boolean);
}

export type ClueGenerator = Generator<string, void, unknown>;

export function mysteryGenerator(answer: string, clue_type: ClueType): ClueGenerator;

export function clueHelp(clue_type: ClueType): string;
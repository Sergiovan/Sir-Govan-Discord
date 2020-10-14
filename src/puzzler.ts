import { ClueType, ClueGenerator, mysteryGenerator, clueHelp } from './secrets';
import { Persist } from './persist';
import {  randomCode, randomEnum, rb_ } from './utils';

import { createHash } from 'crypto';

export class Puzzler {
    answer: string = '';
    puzzle_type: ClueType = ClueType.LetterPosition;
    clue_gen?: ClueGenerator;
    clue_list: string[] = [];
    clue_perm_data: any = null;
    clue_count: number = 0;
    last_clue: Date = new Date(0);
    puzzle_id: string = '';
    puzzle_stopped: boolean = false;

    async load(s: Persist) {
        this.answer = await s.get('answer', '');
        this.puzzle_type = await s.get('puzzle_type', ClueType.LetterPosition);
        this.clue_list = await s.get('clue_list', []);
        this.clue_perm_data = await s.get('clue_perm_data');
        this.clue_count = await s.get('clue_count', 0);
        this.last_clue = new Date(await s.get('last_clue', 0));
        this.puzzle_id = await s.get('puzzle_id', '');
        this.puzzle_stopped = await s.get('puzzle_stopped', false);
    }

    async save(s: Persist) {
        Promise.all([
            s.set('answer', this.answer),
            s.set('puzzle_type', this.puzzle_type),
            s.set('clue_list', this.clue_list),
            s.set('clue_perm_data', this.clue_perm_data),
            s.set('clue_count', this.clue_count),
            s.set('last_clue', this.last_clue.toJSON()),
            s.set('puzzle_id', this.puzzle_id),
            s.set('puzzle_stopped', this.puzzle_stopped)
        ]);
    }

    togglePaused() {
        this.puzzle_stopped = !this.puzzle_stopped;
        return this.puzzle_stopped;
    }

    startClues(): string {
        // if (this.beta) return;

        if (this.puzzle_stopped) {
            return 'Puzzle is paused';
        }

        if (this.answer) { // We already had something going
            this.startGenerator();
            return `Puzzle resumed: \`${this.answer}\`. ID: \`${this.puzzle_id}\`. Puzzle type is: \`${ClueType[this.puzzle_type]}\``;
        }

        // Start from 0
        this.answer = randomCode();
        this.puzzle_type = randomEnum(ClueType);
        this.clue_list = [];
        this.clue_perm_data = null;
        this.clue_count = 0;
        this.startGenerator();

        let hasher = createHash('md5');
        hasher.update(this.answer);
        this.puzzle_id = hasher.digest('hex').substr(0, 16);

        return `Puzzle started: \`${this.answer}\`. ID: \`${this.puzzle_id}\`. Puzzle type is: \`${ClueType[this.puzzle_type]}\``;
    }

    startGenerator() {
        this.clue_gen = mysteryGenerator(this.answer, this.puzzle_type, this.clue_perm_data);
    }

    canGetClue() {
        return !this.puzzle_stopped && (new Date().getTime() - (1000 * 60 * 60) > this.last_clue.getTime()) && this.clue_gen;
    }

    getClue(force: boolean = false): string | null {
        if (!this.canGetClue() && !force) {
            console.log("No clue");
            return null;
        }

        if (this.clue_list.length === 0) {
            for (let i = 0; i < 128; ++i) {
                let clue = this.clue_gen!.next();
                if (clue.done) {
                    this.startGenerator();
                    break;
                } else {
                    this.clue_list.push(clue.value.value);
                    this.clue_perm_data = this.clue_perm_data ?? clue.value.perm_data;
                    if (clue.value.cycle_end) {
                        break;
                    }
                }
            }
        }

        if (this.clue_list.length === 0) {
            this.puzzle_stopped = true;
            console.dir(this);
            throw new Error('Puzzle stopped while calling getClue(), catastrophic error happened');
        }

        let clue = this.clue_list.shift(); 
        this.last_clue = new Date();
        ++this.clue_count;
        return clue!;
    }

    puzzleOngoing() {
        return this.answer?.length && !this.puzzle_stopped;
    }

    checkAnswer(answer: string) {
        if (!this.puzzleOngoing()) {
            return false;
        }

        return answer === this.answer;
    }

    endPuzzle() {
        this.answer = '';
        this.puzzle_type = ClueType.LetterPosition;
        this.clue_list = [];
        this.clue_perm_data = null;
        this.clue_count = 0;
        this.clue_gen = undefined;
        this.puzzle_id = '';
    }

    getHelp(): [boolean, boolean, string] {
        if (!this.answer) {
            return [false, false, ''];
        } else {
            if (this.puzzle_stopped) {
                return [true, false, ''];
            } else {
                return [true, true, clueHelp(this.puzzle_type)];
            }
        }
    }

}
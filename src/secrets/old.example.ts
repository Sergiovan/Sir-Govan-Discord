/* For perusal and archival, old code removed from the secrets */

import { shuffleArray, randomLetter, randomCode, Logger } from '../utils';

export enum ClueType {
    LetterPosition,
    LetterCube,
    BrokenStream,
    Mastermind
};

export class Clue {
    value: string;
    cycle_end: boolean = false;
    perm_data?: any;

    constructor(value: string, cycle_end: boolean = false, perm_data: any = undefined) {
        this.value = value;
        this.cycle_end = cycle_end;
        this.perm_data = perm_data;
    }
};

export type ClueGenerator = Generator<Clue, void, unknown>;

export function * mysteryGenerator(answer: string, clue_type: ClueType, perm_data?: any): ClueGenerator {
    switch (clue_type) {
        case ClueType.LetterPosition: {
            const positions: number[] = [];
            for (let i = 0; i < answer.length; ++i) {
                positions.push(i);
            }

            shuffleArray(positions);

            for (let num of positions) {
                yield new Clue(`${num + 1} ${answer[num]}`);
            }
            return;
        }
        case ClueType.LetterCube: {
            const size = 4;

            if (size * size !== answer.length) {
                yield new Clue('Puzzle broke, send help :(', true);
                return;
            }

            const coords = [...Array(size).keys()];
            const cube_coords = coords.map((y) => coords.map((x) => [x, y]));
            
            let cube; // perm_data[0]
            let shuffled_answer: string[]; // perm_data[1];
            if (!perm_data) {
                cube = ([] as number[][]).concat(...cube_coords);
                shuffleArray(cube);

                shuffled_answer = new Array(16);
                
                cube.forEach((c, i) => {
                    let [x, y] = c;
                    shuffled_answer[x + y * size] = answer[i];
                });
            } else {
                cube = perm_data[0];
                shuffled_answer = perm_data[1];
            }

            let cube_string = '``';
            let i = 0;
            for (let value of shuffled_answer) {
                cube_string += value;
                i++;
                if (i == size) {
                    i = 0;
                    cube_string += '\n';
                } else {
                    cube_string += ' ';
                }
            }
            cube_string += '``';

            while (true) {
                let cube_pos = Math.floor(Math.random() * 16);
                let x = 0, y = 0;
                let i = 0;
                while (i < 16) {
                    if (i === cube_pos) {
                        cube_pos = -1;
                        yield new Clue(cube_string);
                    } else {
                        let [cx, cy] = cube[i];
                        let offx = cx - x; // Left < 0 > Right
                        let offy = cy - y; // Up < 0 > Down
                        let text = '';
                        x = cx;
                        y = cy;
                        if (offx) {
                            text += `${Math.abs(offx)} ${offx < 0 ? 'left' : 'right'}`;
                        }
                        if (offy) {
                            if (offx) {
                                text += ', ';
                            }
                            text += `${Math.abs(offy)} ${offy < 0 ? 'up' : 'down'}`;
                        }
                        if (!offx && !offy) {
                            text = 'Stay put';
                        }
                        i++;
                        yield new Clue(text, i == 16, [cube, shuffled_answer]);
                    }
                }
            }
            return;
        }
        case ClueType.BrokenStream: {
            const max_wrong = 4;
            let i = 0, right = 0, wrong = 0, last_wrong = 0;
            while (i < answer.length) {
                if (!right && !wrong) {
                    if (!last_wrong) {
                        wrong = last_wrong = Math.max(1, Math.ceil(Math.random() * max_wrong));
                        last_wrong = wrong;
                        yield new Clue(`Ignore ${wrong}`);
                    } else {
                        right = Math.min(last_wrong + Math.max(1, Math.ceil(Math.random() * (max_wrong - last_wrong))), answer.length - i);
                        last_wrong = 0;
                        yield new Clue(`Accept ${right}`);
                    }
                } else {
                    if (right) {
                        --right;
                        yield new Clue(`${answer[i++]}`);
                    } else {
                        --wrong;
                        yield new Clue(`${randomLetter()}`);
                    }
                }
            }
            return;
        }
        case ClueType.Mastermind: {
            const dead_value = 3;
            const hurt_value = 2;

            const letter_map = new Set(answer);
            const is_hurt = (guess: string, x: number) => letter_map.has(guess[x]);
            const is_dead = (guess: string, x: number) => answer[x] === guess[x];

            function get_score(guess: string): [number, number[], number[]] {
                let score = 0;
                let dead: number[] = [], hurt: number[] = [];
                for (let i = 0; i < guess.length; ++i) {
                    if (is_dead(guess, i)) {
                        score += dead_value;
                        dead.push(i);
                    } else if (is_hurt(guess, i)) {
                        score += hurt_value;
                        hurt.push(i);
                    }
                }
                return [score, dead, hurt];
            }

            // This function is inexact, but that doesn't matter
            function fix_to_score(guess: string, score: number): string {
                if (score > guess.length * dead_value) {
                    return answer;
                }

                const [guess_score, deads, hurts] = get_score(guess);

                const score_diff = score - guess_score;
                
                if (score_diff <= 1) {
                    return guess;
                }

                let dead = Math.max(0, Math.floor((score_diff / dead_value) / 2));
                let dead_score = dead * dead_value;
                let hurt = 0;
                let hurt_score = 0;

                while (score - (guess_score + dead_score + hurt_score) > 2) {
                    const choose_dead = Math.random() < 0.1;

                    if (choose_dead) {
                        dead++;
                        dead_score += dead_value;
                    } else {
                        hurt++;
                        hurt_score += hurt_value;
                    }
                }

                if (score - (guess_score + dead_score + hurt_score) === 2) {
                    hurt++;
                    hurt_score += hurt_value;
                }
                
                dead += deads.length;
                hurt += hurts.length;
                
                if (dead + hurt > guess.length) {
                    --dead;
                    --hurt;
                }

                dead_score = dead * dead_value;
                hurt_score = hurt * hurt_value;

                const touched = new Set(deads.concat(hurts));
                const untouched = [...Array(guess.length).keys()].filter(x => !touched.has(x));
                shuffleArray(untouched);

                const newdeads = untouched.splice(0, dead - deads.length);
                const newhurt = untouched.splice(0, hurt - hurts.length).concat(hurts);

                for (let nd of newdeads) {
                    guess = guess.substr(0, nd) + answer[nd] + guess.substr(nd + 1);
                    deads.push(nd);
                }

                const taken_letters = new Set(deads.map(x => guess[x]))
                const untaken_letters = new Set(answer.split('').filter(x => !taken_letters.has(x)));

                for (let nh of newhurt) {
                    let letter = '';
                    const choices = Array.from(untaken_letters);
                    shuffleArray(choices);

                    if (untaken_letters.size === 1) {
                        letter = choices[0];
                        if (answer[nh] === letter) {
                            untaken_letters.delete(letter);
                            taken_letters.add(letter);
                            guess = guess.substr(0, nh) + letter + guess.substr(nh + 1);
                            deads.push(nh);
                            continue;
                        }
                    } else if (untaken_letters.size === 0) {
                        continue;
                    } else {
                        letter = choices[0];
                        if (answer[nh] === letter) {
                            letter = choices[1];
                        }
                    }
                    
                    untaken_letters.delete(letter);
                    taken_letters.add(letter);
                    guess = guess.substr(0, nh) + letter + guess.substr(nh + 1);
                    hurts.push(nh);
                }

                return guess;
            }

            let score = 5;

            while (true) {
                let guess = randomCode();
                guess = fix_to_score(guess, score);
                let [guess_score, deads, hurts] = get_score(guess);
                
                {
                    let g = '';
                    for (let i = 0; i < guess.length; ++i) {
                        if (deads.indexOf(i) > -1) {
                            g += guess[i].green;
                        } else if (hurts.indexOf(i) > -1) {
                            g += guess[i].yellow;
                        } else {
                            g += guess[i].red;
                        }
                    }
                }

                score += Math.max(1, Math.floor(Math.random() * (answer.length + 10)) - (answer.length - (deads.length - hurts.length)));
                
                let description = '';
                for (let i = 0; i < guess.length; ++i) {
                    if (is_dead(guess, i)) {
                        description += 'X';
                    } else if (is_hurt(guess, i)) {
                        description += 'x';
                    } else {
                        description += ' ';
                    }
                }
                if (deads.length === guess.length) {
                    description = 'Bingo';
                }

                if (new Set(guess).size !== guess.length) {
                    Logger.warning(">~<");
                }

                yield new Clue(guess);
                yield new Clue(description, guess === answer);
            }
            return;
        }
        default:
            return;
    }
}

export function clueHelp(clue_type: ClueType): string {
    switch (clue_type) {
        case ClueType.LetterPosition:
            return 'Put them in order';
        case ClueType.LetterCube:
            return 'Follow the path. Start top left';
        case ClueType.BrokenStream:
            return 'Order correct, content maybe not';
        case ClueType.Mastermind:
            return 'Play mastermind with me';
    };
}
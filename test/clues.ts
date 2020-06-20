import {randomCode} from '../src/utils';
import {ClueType, ClueGenerator, clueHelp, mysteryGenerator} from '../src/secrets';

function testClues() {
    let code = randomCode();
    console.log(`Testing with code "${code}"`);
    for (let clue_type in ClueType) {
        if (Number.isNaN(Number(clue_type))) {
            continue;
        }
        let enum_val = Number(clue_type) as ClueType;
        console.log(ClueType[clue_type]);
        console.log(enum_val);
        console.log(clueHelp(enum_val));


        let generator = mysteryGenerator(code, enum_val);
        let {value, done} = generator.next();
        while (!done) {
            console.log(value);
            let n = generator.next();
            value = n.value;
            done = n.done;
        }
    }
}

testClues();
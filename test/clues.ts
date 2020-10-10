import { randomCode } from '../src/utils';
import { Clue, ClueType, clueHelp, mysteryGenerator } from '../src/secrets';

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
        let perm_data = null;
        while (!done) {
            let cycle_end = (value as Clue).cycle_end;
            console.log((value as Clue).value);
            perm_data = perm_data ?? ((value as Clue).perm_data);
            let n = generator.next();
            value = n.value as Clue;
            done = n.done;
            if (cycle_end) {
                console.log('Cycle end');
                generator.return();
                break;
            }
        }

        if (perm_data) {
            console.log('Second run with perm_data from previous run');
            console.dir(perm_data);
            let generator = mysteryGenerator(code, enum_val, perm_data);
            let {value, done} = generator.next();

            while (!done) {
                let cycle_end = (value as Clue).cycle_end;
                console.log((value as Clue).value);
                let n = generator.next();
                value = n.value as Clue;
                done = n.done;
                if (cycle_end) {
                    console.log('Cycle end');
                    generator.return();
                    break;
                }
            }
        }
    }
}

testClues();
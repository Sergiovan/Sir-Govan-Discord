import * as D from 'discord.js';
import {discord} from '../src/secrets/secrets';
import * as fs from 'fs';
import * as path from 'path';
import * as util from 'util';

type PreparationEntry = {
    message: D.Snowflake;
    channel: D.Snowflake;
    original_message: D.Snowflake;
    original_channel: D.Snowflake;
    pins: number;
    funny: string;
    ignore: boolean;
    _comment: string;
};

type Preparation = {
    entries: PreparationEntry[];
};

type Contestant = {
    entry: number;
    votes: number;
}

type Battle = {
    A: Contestant;
    B: Contestant;
};

type Round = {
    totals: {[key: number]: number};
    battles: Battle[];
};

type DismantledEmbed = {
    color: number;
    author_name: string;
    author_avatar: string;

    message: D.Snowflake;
    channel: D.Snowflake;

    content: string;
    image: string;
};

const footer_regex = /(\d+) - (\d+)/;
const tournament_directory = path.join(__dirname, '..', '..', 'tournaments');
const tournament_channel = '927184449974718544'; // Real: '927184449974718544' Fake: '216992217988857857'
const a_emoji = "🅰️";
const a_url = 'https://twemoji.maxcdn.com/v/latest/72x72/1f170.png';
const b_emoji = "🅱️";
const b_url = 'https://twemoji.maxcdn.com/v/latest/72x72/1f171.png';

async function fetch_messages(channel: D.TextChannel, after: D.Snowflake, until: D.Snowflake) {
    after = `${BigInt(after) - 1n}`;
    console.log(`Fetching messages from ${channel.name}: ${after} to ${until}`)
    let res: D.Message[] = [];
    while (true) {

        let logs: D.Message[] = Array.from(
            (await channel.messages.fetch({after: res[0]?.id ?? after, limit: 100})).values()
        );
        
        const last = logs[logs.length - 1];
        logs = logs.filter((m) => BigInt(m.id) <= BigInt(until));

        if (!logs.length || BigInt(last.id) >= BigInt(until)) {
            break;
        }

        res = logs.concat(res);
        process.stdout.write('.')
    }
    process.stdout.write('\x1b[2K\rDone\n');
    return res;
}

function dismantle_embed(msg: D.Message): DismantledEmbed {
    console.log(`Dismantling ${msg.id}`);
    const ret: DismantledEmbed = {
        color: 0, 
        author_name: '', 
        author_avatar: '', 
        channel: '', 
        message: '',
        content: '',
        image: '',
    };

    if (!msg.embeds || !msg.embeds.length) {
        console.error(`Message ${msg.id} didn't have an embed`);
        return ret;
    }

    const embed = msg.embeds[0];
    ret.color = embed.color ?? 0xc0ffee;
    if (!embed.author || !embed.author.name || !embed.author.iconURL) {
        console.error(`Message ${msg.id} has no author`);
        return ret;
    }

    ret.author_name = embed.author.name;
    ret.author_avatar = embed.author.iconURL;

    if (!embed.footer || !embed.footer.text) {
        console.error(`Message ${msg.id} has no footer`);
        return ret;
    }

    const regex_result = footer_regex.exec(embed.footer.text);
    if (!regex_result) {
        console.error(`Footer of message ${msg.id} does not conform: ${embed.footer.text}`);
        return ret;
    }

    const [_, original_msg, original_channel, ...rest] = regex_result;
    ret.message = original_msg;
    ret.channel = original_channel;

    if (!embed.description && !embed.fields[0]) {
        console.error(`Message ${msg.id} has no description`);
        return ret;
    }

    ret.content = embed.description || embed.fields[0].value;

    ret.image = embed.image?.url ?? embed.thumbnail?.url ?? '';

    console.log(`Dismantled ${msg.id}`);
    return ret;
}

async function create_entry(clnt: D.Client, embed_msg: D.Message, embed_data: DismantledEmbed, emoji: D.MessageReactionResolvable) {
    console.log(`Creating entry for ${embed_msg.id}`);
    const ret: PreparationEntry = {
        funny: '',
        ignore: false,
        channel: embed_msg.channelId,
        message: embed_msg.id,
        original_channel: embed_data.channel,
        original_message: embed_data.message,
        pins: 4,
        _comment: ` ${embed_data.content} ${embed_data.image} `,
    };
    
    const original_channel = await clnt.channels.resolve(embed_data.channel);

    if (!original_channel) {
        throw new Error(`Channel ${embed_data.channel} cannot be found`);
    }

    if (!original_channel.isTextBased()) {
        throw new Error(`Channel ${original_channel.name} is not a text channel`);
    }

    if (original_channel.isDMBased()) {
        throw new Error(`Channel ${original_channel.id} is just DMs, actually?`);
    }

    const original_msg = await original_channel.messages.fetch(embed_data.message);

    if (!original_msg) {
        console.error(`Channel ${original_channel.name} has no message with id ${embed_data.message}. May have been deleted, continuing`);
        return ret; // 4 pins is the default
    }

    const reactions = await original_msg.reactions.resolve(emoji);
    if (!reactions) {
        console.error(`Message ${original_msg.id} has no reactions for ${emoji}. Assuming 4`);
        return ret;
    }

    ret.pins = reactions.count;
    console.log(`Entry created for ${embed_msg.id}`);
    return ret;
}

async function create_battle(clnt: D.Client, tournament: string, round_nr: number, round: Round, battle: number, data: Preparation): Promise<D.MessageCreateOptions> {
    async function create_embed(entry_number: number, a: boolean) {
        const name = a ? a_emoji : b_emoji;
        const footer = `${tournament} tournament | Round ${round_nr} | Battle ${battle + 1} | Entry ${a ? 'A' : 'B'}`;

        console.log(`Creating embed for "${footer}"`)

        if (entry_number >= data.entries.length) {
            throw new Error(`Battle ${name} (${entry_number}) does not exist`);
        }

        const entry = data.entries[entry_number];

        const channel = await clnt.channels.fetch(entry.channel);
        if (!channel || !channel.isTextBased() || channel.isDMBased()) {
            throw new Error(`Cannot find channel ${entry.channel} in battle ${name}: ${entry_number}`);
        }

        const message = await channel.messages.fetch(entry.message);
        if (!message) {
            throw new Error(`Cannot find message ${entry.message} in #${channel.name}  in battle ${name}: ${entry_number}`);
        }

        const original_channel = await clnt.channels.fetch(entry.original_channel);
        if (!original_channel || !original_channel.isTextBased() || original_channel.isDMBased()) {
            throw new Error(`Cannot find channel ${entry.original_channel} in battle ${name}: ${entry_number}`);
        }
    
        const original_message = await original_channel.messages.fetch(entry.original_message);
        if (!original_message) {
            throw new Error(`Cannot find message ${entry.message} in #${original_channel.name}  in battle ${name}: ${entry_number}`);
        }
    
        const original_embed = dismantle_embed(message);
    
        if (!original_embed.content) {
            throw new Error(`Could not parse embed for ${original_message.id} in battle ${name}: ${entry_number}`);
        }

        const content = original_embed.content.replace(/\[Click to teleport\]\(.*?\)/, '');

        const embed = new D.EmbedBuilder();
        try {
            embed.setTitle(entry.funny)
                .setColor(original_embed.color)
                .setURL(original_message.url)
                .setAuthor({
                    name: original_message.author.tag, 
                    iconURL: original_message.author.avatarURL({extension: 'png', forceStatic: true}) || original_message.author.defaultAvatarURL
                })
                .setThumbnail(a ? a_url : b_url)
                .setImage(original_embed.image || null)
                .setDescription(content || null)
                .setFooter({text: footer});
        } catch (err) {
            console.error(util.inspect(entry));
            throw err;
        }

        return embed;
    }

    // Sometimes entry B just straight up doesn't exist lmfao
    if (round.battles[battle].B.entry !== -1) {
        return {
            content: `Battle #${battle + 1}`,
            embeds: await Promise.all([create_embed(round.battles[battle].A.entry, true), create_embed(round.battles[battle].B.entry, false)])
        }; 
    } else {
        return {
            content: `Battle #${battle + 1}. No entry ${b_emoji}.`,
            embeds: [await create_embed(round.battles[battle].A.entry, true)]
        };
    }
}

async function find_round_messages(clnt: D.Client, tournament_name: string, round: number) {
    const post_channel = await clnt.channels.fetch(tournament_channel);
            
    if (!post_channel || !post_channel.isTextBased() || post_channel.isDMBased()) {
        console.error(`${tournament_channel} is not available`);
        return [];
    }
    
    let last: D.Snowflake = `${BigInt((await post_channel.messages.fetch({limit: 1})).at(0)!.id) + 1n}`;

    const footer_regex = /(?<tournament>.*) tournament \| Round (?<round>\d+) \| Battle (?<battle>\d+) \| Entry (?<entry>A|B)/;

    let found: D.Message[] = [];
    let loop_found_any = true;

    while (loop_found_any) {
        loop_found_any = false;
        let msgs: D.Message[] = Array.from(
            (await post_channel.messages.fetch({limit: 50, before: last})).values()
        );
        last = msgs[msgs.length - 1].id;

        msg_loop: for (let msg of msgs) {
            if (msg.author.id !== clnt.user!.id) continue; // Not mine
            if (!msg.embeds.length) continue; // No embeds
            
            for (let embed of msg.embeds) {
                if (!embed.footer) continue msg_loop;

                const res = footer_regex.exec(embed.footer.text);
                if (!res || !res.groups) continue msg_loop;

                // console.log(`Candidate ${msg.id}: Tournament "${res.groups.tournament}" round #${res.groups.round}. Battle ${res.groups.battle}, Entry ${res.groups.entry}`);

                if (res.groups.tournament !== tournament_name) continue msg_loop;
                if (res.groups.round !== `${round}`) continue msg_loop;
            }

            // console.log(`Found ${msg.id}`);
            found.push(msg);
            loop_found_any = true;
        }
    }

    return found;
}

function get_winner(battle: Battle, previous_round: Round, preparation: Preparation) {
    const A = battle.A;
    const B = battle.B;

    if (B.entry === -1) {
        return A;
    } else if (A.entry === -1) {
        return B;
    } else if (A.votes > B.votes) {
        return A;
    } else if (B.votes > A.votes) {
        return B;
    } else if (preparation.entries[A.entry].pins > preparation.entries[B.entry].pins) {
        return A;
    } else if (preparation.entries[B.entry].pins > preparation.entries[A.entry].pins) {
        return B;
    } else if (previous_round.totals[A.entry] > previous_round.totals[B.entry]) {
        return A;
    } else if (previous_round.totals[B.entry] > previous_round.totals[A.entry]) {
        return B;
    } else if (A.entry < B.entry) {
        return A;
    } else {
        return B;
    }
}

function tourney_dir(tournament_name: string) {
    return path.join(tournament_directory, tournament_name);
}

function prepare_doc(tournament_name: string) {
    return path.join(tourney_dir(tournament_name), 'prepare.json');
}

function round_doc(tournament_name: string, round: number) {
    return path.join(tourney_dir(tournament_name), `round_${round}.json`)
}

function shuffle<T>(array: T[]) {
    for (let i = array.length - 1; i > 0; i--) {
        let j = Math.floor(Math.random() * (i + 1));
        let temp = array[i];
        array[i] = array[j];
        array[j] = temp;
    }
}


// node tournaments.js prepare <channel-id> <message-first> <message-last> <tournament-name> <pin-emoji>
// node tournaments.js post-prepare <tournament-name>
// node tournaments.js round <tournament-name> <round>
// node tournaments.js check <tournament-name> <round>
// node tournaments.js count <tournament-name> <round> 
// node tournaments.js clean <tournament-name> <round>
// node tournaments.js winner <tournament-name> <round>

const params = process.argv.slice(2);
const [sort, ..._] = params; 

if (!fs.existsSync(tournament_directory)) {
    fs.mkdirSync(tournament_directory);
}

const Flags = D.GatewayIntentBits;
const Partials = D.Partials;
let client = new D.Client({
    intents: [
        Flags.Guilds,
        Flags.GuildMembers,
        Flags.GuildVoiceStates,
        Flags.GuildPresences,
        Flags.GuildMessages,
        Flags.GuildMessageReactions,
        Flags.DirectMessages,
        Flags.DirectMessageReactions,
        Flags.MessageContent,
    ],
    partials: [
        Partials.Message,
        Partials.Channel,
        Partials.Reaction
    ]
});

client.on('ready', async function(this: D.Client) {
    const self = this;
    switch (sort) {
        case 'prepare': {
            const prepare_params = params.slice(1);
            if (prepare_params.length !== 5) {
                console.error(`Parameters were not right: ${prepare_params}`);
                break;
            }

            const [embed_channel, message_first, message_last, tournament_name, pin_emoji] = prepare_params;
            
            const tourn_path = tourney_dir(tournament_name);
            const prepare_file = prepare_doc(tournament_name);
            const round0_file = round_doc(tournament_name, 0);

            if (!fs.existsSync(tourn_path)) {
                fs.mkdirSync(tourn_path);
            }

            const results: Preparation = {entries: []};
            
            if (!fs.existsSync(prepare_file)) {
                const channel = await this.channels.fetch(embed_channel);

                if (!channel || !(channel instanceof D.TextChannel)) {
                    throw new Error(`Invalid channel ${embed_channel}`);
                }

                const msgs = (await fetch_messages(channel, message_first, message_last)).reverse();
                results.entries = await Promise.all(msgs.map((msg) => create_entry(self, msg, dismantle_embed(msg), pin_emoji)));

                fs.writeFileSync(prepare_file, JSON.stringify(results, null, 2));
                if (fs.existsSync(round0_file)) {
                    fs.rmSync(round0_file, {force: false});
                }
            } else {
                console.error(`${tourn_path} already exist`);
            }

            break;
        }
        case 'post-prepare': {
            const prepare_params = params.slice(1);
            if (prepare_params.length !== 1) {
                console.error(`Parameters were not right: ${prepare_params}`);
                break;
            }

            const [tournament_name] = prepare_params;

            const round0_file = round_doc(tournament_name, 0);
            const prepare_file = prepare_doc(tournament_name);
            
            const results: Preparation = {entries: []};

            if (!fs.existsSync(round0_file)) {
                results.entries = JSON.parse(fs.readFileSync(prepare_file, 'utf8')).entries;
                
                let entries = [...Array(results.entries.length).keys()];
                entries = entries.filter((n) => !results.entries[n].ignore)
                
                if (!entries.length) {
                    console.error(`There were no entries to load`);
                    break;
                }

                shuffle(entries);
                
                if (entries.length % 2) {
                    entries.push(-1);
                } 

                const round: Round = {totals: {}, battles: []};

                for (let i = 0; i < entries.length / 2; ++i) {
                    const a = entries[i * 2];
                    const b = entries[i * 2 + 1];
                    round.battles.push({A: {entry: a, votes: 0}, B: {entry: b, votes: 0}});
                    round.totals[a] = 0;
                    round.totals[b] = 0;
                }

                fs.writeFileSync(round0_file, JSON.stringify(round, null, 2));

                break;
            } else {
                console.error(`${round0_file} already exists`);
            }

            break;
        }
        case 'round': {
            const prepare_params = params.slice(1);
            if (prepare_params.length !== 2) {
                console.error(`Parameters were not right: ${prepare_params}`);
                break;
            }

            const post_channel = await this.channels.fetch(tournament_channel);
            
            if (!post_channel || !post_channel.isTextBased()) {
                console.error(`${tournament_channel} is not available`);
                break;
            }

            const [tournament_name, round_name] = prepare_params;
            
            const round = Number.parseInt(round_name);

            if (Number.isNaN(round)) {
                console.error(`"${round_name}" could not be parsed to a number`);
                break;
            }

            const prepare_file = prepare_doc(tournament_name);
            const prev_round_file = round_doc(tournament_name, round - 1);
            const round_file = round_doc(tournament_name, round);

            if (!fs.existsSync(prepare_file)) {
                console.error(`Tournament ${tournament_name} has not been prepared, aborting`);
                break;
            }

            if (!fs.existsSync(prev_round_file)) {
                console.error(`Round ${round - 1} has not been post-prepared`);
                break;
            }

            if (fs.existsSync(round_file)) {
                console.error(`Round ${round} has already been posted`);
                break;
            }

            const preparation: Preparation = JSON.parse(fs.readFileSync(prepare_file, 'utf-8'));
            const prev_round: Round = JSON.parse(fs.readFileSync(prev_round_file, 'utf-8'));

            let current_round: Round;

            if (round - 1 === 0) {
                current_round = prev_round;
            } else {
                const totals: {[key: number]: number} = {};
                const winners: Contestant[] = prev_round.battles.map((battle) => {
                    const winner = get_winner(battle, prev_round, preparation);
                    totals[winner.entry] = (prev_round.totals[winner.entry] ?? 0) + winner.votes;

                    return {entry: winner.entry, votes: 0};
                });

                const battles: Battle[] = [];

                if (winners.length % 2 === 1) {
                    winners.push({entry: -1, votes: 0});
                }

                for (let i = 0; i < winners.length / 2; ++i) {
                    const a = winners[i * 2];
                    const b = winners[i * 2 + 1];
                    battles.push({A: a, B: b});
                }

                current_round = {totals: totals, battles: battles};
            }

            let msgs = await Promise.all(current_round.battles.map((_, i) => create_battle(self, tournament_name, round, current_round, i, preparation)));

            let real_msgs: D.Message[] = [];

            let start = await post_channel.send({
                content: `Round __#${round}__ of the **${tournament_name}** tournament.\n` + 
                         `${msgs.length} matches, with ${(msgs.length * 2) - (current_round.totals[-1] !== undefined ? 1 : 0)} entries.`
            });

            for (let msg of msgs) {
                real_msgs.push(await post_channel.send(msg));
            }

            let end = await post_channel.send({
                content: `This is the end of the entries for round __#${round}__ of the **${tournament_name}** tournament.\n` + 
                         `To go back to the start of the round hitch a ride on this url: ${start.url}`
            });

            await Promise.all(real_msgs.map(async (msg) => {
                await msg.react(a_emoji);
                return await msg.react(b_emoji);
            }));

            fs.writeFileSync(round_file, JSON.stringify(current_round, null, 2));

            break;
        }
        case 'check': {
            const prepare_params = params.slice(1);
            if (prepare_params.length !== 2) {
                console.error(`Parameters were not right: ${prepare_params}`);
                break;
            }

            const [tournament_name, round_name] = prepare_params;

            const round = Number.parseInt(round_name);

            if (Number.isNaN(round)) {
                console.error(`"${round_name}" could not be parsed to a number`);
                break;
            }

            let entries = (await find_round_messages(self, tournament_name, round)).reverse();

            let missing = 0;

            await Promise.all(entries.map(async (msg, i) => {
                const a_reactions = msg.reactions.resolve(a_emoji);
                const b_reactions = msg.reactions.resolve(b_emoji);
                if (!a_reactions || !a_reactions.me) {
                    console.log(`Entry ${i} was missing an ${a_emoji} reaction`);
                    missing++;
                    // await msg.react(a_emoji);
                }
                if (!b_reactions || !b_reactions.me) {
                    console.log(`Entry ${i} was missing a ${b_emoji} reaction`);
                    missing++;
                    // await msg.react(b_emoji);
                }
            }));

            console.log(`Missed ${missing}/${entries.length * 2} reactions`);

            break;
        }
        case 'count': {
            const prepare_params = params.slice(1);
            if (prepare_params.length !== 2) {
                console.error(`Parameters were not right: ${prepare_params}`);
                break;
            }

            const [tournament_name, round_name] = prepare_params;

            const round = Number.parseInt(round_name);

            if (Number.isNaN(round)) {
                console.error(`"${round_name}" could not be parsed to a number`);
                break;
            }

            const round_file = round_doc(tournament_name, round);
            const round_data: Round = JSON.parse(fs.readFileSync(round_file, 'utf-8'));

            let entries = (await find_round_messages(self, tournament_name, round)).reverse();

            entries.forEach((msg, i) => {
                const battle = round_data.battles[i];
                const as = msg.reactions.resolve(a_emoji)?.count ?? 1;
                const bs = msg.reactions.resolve(b_emoji)?.count ?? 1;

                // Remove govan vote
                battle.A.votes = as - 1;
                battle.B.votes = bs - 1;
            });

            fs.writeFileSync(round_file, JSON.stringify(round_data, null, 2));

            break;
        }
        case 'clean': {
            const prepare_params = params.slice(1);
            if (prepare_params.length !== 2) {
                console.error(`Parameters were not right: ${prepare_params}`);
                break;
            }

            const [tournament_name, round_name] = prepare_params;
            
            const round = Number.parseInt(round_name);

            if (Number.isNaN(round)) {
                console.error(`"${round_name}" could not be parsed to a number`);
                break;
            }

            let deletion = await find_round_messages(self, tournament_name, round);

            await Promise.all(deletion.map((msg) => msg.delete()));
            console.log(`Deletion complete`);
            break;
        }
        case 'winner': {
            const prepare_params = params.slice(1);
            if (prepare_params.length !== 2) {
                console.error(`Parameters were not right: ${prepare_params}`);
                break;
            }

            const [tournament_name, round_name] = prepare_params;
            
            const round = Number.parseInt(round_name);

            if (Number.isNaN(round)) {
                console.error(`"${round_name}" could not be parsed to a number`);
                break;
            }

            const prepare_file = prepare_doc(tournament_name);
            const round_file = round_doc(tournament_name, round);

            const preparation: Preparation = {entries: JSON.parse(fs.readFileSync(prepare_file, 'utf8')).entries};
            const round_data: Round = JSON.parse(fs.readFileSync(round_file, 'utf-8'));

            if (round_data.battles.length !== 1) {
                console.error("To declare a winner you need exactly 1 battle");
                break;
            }

            let winner = preparation.entries[get_winner(round_data.battles[0], round_data, preparation).entry];

            console.log(`Winner is: ${winner.funny}`);

            break;
        }
        default:
            console.error(`Invalid first argument, try one of "prepare", "round", "count" or "clean"`);
            break;
    }

    this.destroy();
});

client.login(discord.token);
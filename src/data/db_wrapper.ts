import * as D from 'discord.js';

import * as db from './db';

class DBProxy<T extends db.DBName, U extends db.DBRecord<T>> {
    table: T;
    rowid: number;
    db: db.DB;
    to_commit: Partial<U> = {};
    
    constructor(table: T, rowid: number, db: db.DB) {
        this.table = table;
        this.rowid = rowid;
        this.db = db;
    }

    get(target: U, key: keyof U | 'commit') {
        if (key === 'commit') {
            return () => {
                if (Object.keys(this.to_commit).length === 0) {
                    return;
                }
                // This can take some time, that's ok
                this.db.conn(this.table).update(this.to_commit).where('rowid', this.rowid);
                for (let element in this.to_commit) {
                    if (this.to_commit[element] !== undefined) {
                        target[element] = this.to_commit[element]!; // Could be null, that's okay
                    }
                }
                this.to_commit = {};
            };
        } else {
            return target[key];
        }
    }

    set<V extends keyof U>(target: U, key: V, value: U[V]) {
        if (key === 'rowid') {
            throw new Error('Cannot modify rowid');
        } else {
            if (target[key] !== value) {
                this.to_commit[key] = value;
            }
            return true;
        }
    }
}

type Committable<T> = T & {commit: () => Promise<void>}
type MaybeProxy<T> = Promise<Committable<T> | undefined>
type JustProxy<T> = Promise<Committable<T>>;

export type DBUserProxy = Committable<db.User>;

export class DBWrapper {
    db: db.DB;

    constructor(database_location: string) {
        this.db = new db.DB(database_location);
    }

    close() {
        this.db.close();
    }

    private proxify<T extends db.DBName, U extends db.DBRecord<T>>(table: T, data: U): Committable<U> {
        // The casts work because Committable<U> is stricter than ProxyHandler<U> and won't let the lax typing slip
        return new Proxy(data, new DBProxy(table, data.rowid, this.db) as ProxyHandler<U>) as Committable<U>;
    }

    private async add<T extends db.DBName>(table: T, data: Partial<db.DBRecord<T>>) {
        const rowid = await this.db.conn(table).insert(data);
        data.rowid = rowid[0];
        
        return this.proxify(table, data as db.DBRecord<T>);
    }

    async getAllUsers(): Promise<Committable<db.User>[]> {
        return (await this.db.get('users', {}))
            .map((e: db.DBRecord<'users'>) => new Proxy(e, new DBProxy('users', e.rowid, this.db)) as Committable<db.User>);
    }

    async addUser(user: D.User, member: number, uninterested: number, nickname: string | null = null): JustProxy<db.User> {
        const data: Partial<db.User> = {
            id: user.id,
            name: user.username,
            discriminator: user.discriminator,
            nickname: nickname,
            avatar: user.avatar ? user.avatarURL()! : user.defaultAvatarURL,
            xp: 0,
            xp_total: 0,
            level: 0,
            is_member: member,
            option_uninterested: uninterested
        };

        return this.add('users', data);
    }

    async getUser(user: D.User): MaybeProxy<db.User>;
    async getUser(user: D.Snowflake): MaybeProxy<db.User>;
    async getUser(user: D.User | D.Snowflake): MaybeProxy<db.User> {
        let id: D.Snowflake;
        if (user instanceof D.User) {
            id = user.id;
        } else {
            id = user;
        }

        const obj = await this.db.getFirst('users', {id: id});
        if (obj) {
            return this.proxify('users', obj);
        } else {
            return obj;
        }
    }

    async addPuzzle(data: Partial<db.Puzzle>): JustProxy<db.Puzzle> {
        return this.add('puzzles', data);
    }

    async getPuzzle(id: string): MaybeProxy<db.Puzzle> {
        const obj = await this.db.getLast('puzzles', {id: id});
        if (obj) {
            return this.proxify('puzzles', obj);
        } else {
            return obj;
        }
    }

    async addClue(puzzle: string, msg: D.Message): JustProxy<db.Clue> {
        const puzzle_data = await this.getPuzzle(puzzle);
        if (!puzzle_data) {
            throw Error(`Puzzle ${puzzle} doesn't exist in database`);
        }
        const data: Partial<db.Clue> = {
            puzzle_id: puzzle_data.rowid,
            message_id: msg.id,
            content: msg.content,
            created_time: new Date(msg.createdAt)
        };
        return this.add('clues', data);
    }

    async getClue(msg: D.Message) {
        const obj = await this.db.getFirst('clues', {message_id: msg.id});
        if (obj) {
            return this.proxify('clues', obj);
        } else {
            return obj;
        }
    }

    async addClueSteal(msg: D.Message, user: D.User) {
        const clue_data = await this.getClue(msg);
        if (!clue_data) {
            throw Error(`Clue for message ${msg.id} does not exist in database`);
        }
        const user_data = await this.getUser(user);
        if (!user_data) {
            throw Error(`User ${user.username} does not exist in database`);
        }

        const data: Partial<db.ClueSteal> = {
            clue_id: clue_data.rowid,
            user_id: user_data.rowid,
            steal_time: new Date()
        };

        return this.add('clue_steals', data);
    }

    async transferXP(from: D.User | null, to: D.User | null, amount: number, reason: number) {
        if (!from && !to) {
            throw new Error("Cannot have XP transaction without users");
        }

        let from_data, to_data;
        if (from) {
            from_data = await this.getUser(from);
            if (!from_data) {
                throw Error(`User ${from.username} does not exist in database`);
            }
        } else {
            from_data = null;
        }
        if (to) {
            to_data = await this.getUser(to);
            if (!to_data) {
                throw Error(`User ${to.username} does not exist in database`);
            }
        } else {
            to_data = null;
        }

        const data: Partial<db.XpTransaction> = {
            user_sender: from_data?.rowid ?? null,
            user_receiver: to_data?.rowid ?? null,
            amount: amount,
            reason: reason,
            transaction_time: new Date()
        };

        return this.add('xp_transactions', data);
    }
};
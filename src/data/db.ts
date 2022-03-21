import { Snowflake } from 'discord.js';
import { knex, Knex } from 'knex';

interface Table {
    rowid: number;
}

export interface User extends Table {
    id: Snowflake;
    name: string;
    discriminator: string;
    nickname: string | null;
    avatar: string;
    xp: number;
    xp_total: number;
    level: number;
    is_member: number;
    option_uninterested: number;
}

export interface Puzzle extends Table {
    id: string;
    answer: string;
    type: number;
    started_time: Date;
    ended_time: Date | null;
    winner: number | null;
}

export interface Clue extends Table {
    puzzle_id: number;
    message_id: string;
    content: string;
    created_time: Date;
}

export interface ClueSteal extends Table {
    clue_id: number;
    user_id: number;
    steal_time: Date;
}

export interface XpTransaction extends Table {
    user_sender: number | null;
    user_receiver: number | null;
    amount: number;
    reason: number;
    transaction_time: Date;
}

export type DBName = 'users' | 'puzzles' | 'clues' | 'clue_steals' | 'xp_transactions';
export type DBRecord<T extends DBName> = 
    T extends 'users' ? User :
    T extends 'puzzles' ? Puzzle :
    T extends 'clues' ? Clue :
    T extends 'clue_steals' ? ClueSteal :
    T extends 'xp_transactions' ? XpTransaction : never;

// These type shenanigans are so knex will take my parameters without throwing a hissy fit
// Note: I am unsure of what this actually does, or how to do it better
// If you are reading this and you do please shoot me a mail or pull request, thx
type AnyOrUnknownToOther<T1, T2> = unknown extends T1 ? T2 : T1;
export type KnexApproved<T> = Readonly<Readonly<Partial<AnyOrUnknownToOther<Knex.MaybeRawRecord<T>, {}>>>>;

export class DB {
    conn: Knex;

    constructor(db_file: string) {
        this.conn = knex({
            client: 'better-sqlite3',
            connection: {
                filename: db_file,
            },
            useNullAsDefault: true
        });
    }

    async insert<T extends DBName>(table: T, value: KnexApproved<DBRecord<T>>) {
        return await this.table(table).insert(value as any).select('rowid', '*');
    }

    async update<T extends DBName>(table: T, where: Partial<DBRecord<T>>, update: KnexApproved<DBRecord<T>>) {
        return await this.table(table).where(where).update(update as any);
    }

    async delete<T extends DBName>(table: T, where: Partial<DBRecord<T>>) {
        return await this.table(table).where(where).del();
    }

    async get<T extends DBName>(table: T, where: Partial<DBRecord<T>>) {
        return await this.table(table).where(where).select('rowid', '*');
    }

    async getFirst<T extends DBName>(table: T, where: Partial<DBRecord<T>>) {
        return await this.table(table).where(where).select('rowid', '*').first();
    }

    async getLast<T extends DBName>(table: T, where: Partial<DBRecord<T>>) {
        let elems = await this.table(table).select('rowid', '*').where(where);
        if (elems.length) {
            return elems[elems.length - 1];
        } else {
            return undefined;
        }
    }

    async select<T extends DBName>(table: T, where: Partial<DBRecord<T>>, select: (keyof DBRecord<T>)[] = []) {
        return await this.table(table).select(select.length ? select : '*', 'rowid').where(where);
    }

    table<T extends DBName>(table: T): Knex.QueryBuilder<DBRecord<T>> {
        return this.conn<DBRecord<T>>(table);
    }

    close() {
        this.conn.destroy();
    }

};
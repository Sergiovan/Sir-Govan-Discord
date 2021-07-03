import * as D from 'discord.js';

import { DBUserProxy } from '../data/db_wrapper';
import { xp } from '../secrets/secrets';

/**
 * Wrapper for a discord user, with extra data 
 */
export class BotUser {
    db_user: DBUserProxy; // Database proxy for users
    last_spoke: number; // Timestamp for last time this user spoke

    constructor(db_user: DBUserProxy) {
        this.db_user = db_user;
        this.last_spoke = Date.now();
    }

    /** If this user wants shenanigans */
    allow(): boolean {
        return this.db_user.is_member > 0 && !this.db_user.option_uninterested;
    }

    /** Updates the db user with a real Eris user */
    update_user(user: D.User) {
        this.db_user.name = user.username;
        this.db_user.discriminator = user.discriminator;
        this.db_user.avatar = user.displayAvatarURL();
    }

    /** Updates the db user with a real Eris guild member */
    update_member(member: D.GuildMember) {
        this.db_user.is_member = 1;
        this.db_user.name = member.user.username;
        this.db_user.discriminator = member.user.discriminator;
        this.db_user.avatar = member.user.displayAvatarURL();
        this.db_user.nickname = member.nickname;
    }

    /** Adds or removes xp from a user */
    change_xp(amount: number) {
        if (amount > 0) {
            return this.add_xp(amount);
        } else if (amount < 0) {
            return this.remove_xp(-amount);
        } else {
            return true;
        }
    }

    /** Adds XP to a user */
    add_xp(amount: number) {
        this.db_user.xp += amount;
        this.db_user.xp_total += amount;
        this.db_user.level = xp.XpToLevel(this.db_user.xp_total + amount);
        return true;
    }

    /** Removes XP from a user, if possible */
    remove_xp(amount: number) {
        if (this.db_user.xp < amount) {
            return false;
        } else {
            this.db_user.xp -= amount;
            return true;
        }
    }

    /** Commits this user's data to the database */
    commit() {
        this.db_user.commit();
    }
}
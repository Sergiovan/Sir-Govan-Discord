import Eris from 'eris';

import { DBUserProxy } from '../data/db_wrapper';
import { xp } from '../secrets/secrets';

export class BotUser {
    db_user: DBUserProxy;
    last_spoke: number;

    constructor(db_user: DBUserProxy) {
        this.db_user = db_user;
        this.last_spoke = Date.now();
    }

    allow(): boolean {
        return this.db_user.is_member > 0 && !this.db_user.option_uninterested;
    }

    update_user(user: Eris.User) {
        this.db_user.name = user.username;
        this.db_user.discriminator = user.discriminator;
        this.db_user.avatar = user.avatar ? user.avatarURL : user.defaultAvatarURL;
    }

    update_member(member: Eris.Member) {
        this.db_user.is_member = 1;
        this.db_user.name = member.username;
        this.db_user.discriminator = member.discriminator;
        this.db_user.avatar = member.avatar ? member.avatarURL : member.defaultAvatarURL;
        this.db_user.nickname = member.nick ?? null;
    }

    change_xp(amount: number) {
        if (amount > 0) {
            return this.add_xp(amount);
        } else if (amount < 0) {
            return this.remove_xp(-amount);
        } else {
            return true;
        }
    }

    add_xp(amount: number) {
        this.db_user.xp += amount;
        this.db_user.xp_total += amount;
        this.db_user.level = xp.XpToLevel(this.db_user.xp_total + amount);
        return true;
    }

    remove_xp(amount: number) {
        if (this.db_user.xp < amount) {
            return false;
        } else {
            this.db_user.xp -= amount;
            return true;
        }
    }

    commit() {
        this.db_user.commit();
    }
}
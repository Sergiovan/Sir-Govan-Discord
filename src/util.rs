use serenity::model::prelude::*;
use serenity::prelude::*;

use serenity::utils::Colour;

pub mod logger;

pub enum UniqueColorError {
    GuildMissing,
    RolesMissing,
    NoColoredRole,
}

pub trait ResultErrorHandler<T> {
    fn log_if_err(self, msg: &str);
    fn log_or_option(self, msg: &str) -> Option<T>;
}

impl<T, E: ::std::fmt::Display> ResultErrorHandler<T> for Result<T, E> {
    fn log_if_err(self, msg: &str) {
        match self {
            Ok(_) => (),
            Err(e) => {
                logger::error(&format!("{}: {}", msg, e));
            }
        }
    }

    fn log_or_option(self, msg: &str) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                logger::error(&format!("{}: {}", msg, e));
                None
            }
        }
    }
}

pub trait NickOrName {
    fn get_name(&self) -> &str;
}

impl NickOrName for Member {
    fn get_name(&self) -> &str {
        self.nick.as_ref().unwrap_or(&self.user.name)
    }
}

pub fn get_unique_color(ctx: &Context, member: &Member) -> Result<Role, UniqueColorError> {
    let guild = match ctx.cache.guild(member.guild_id) {
        Some(g) => g,
        None => return Err(UniqueColorError::GuildMissing),
    };

    let mut roles = match member.roles(ctx) {
        Some(r) => r,
        None => return Err(UniqueColorError::RolesMissing),
    };

    roles.sort_by_key(|r| r.position);

    for role in roles.iter().rev() {
        if role.colour == Colour(0) {
            continue;
        }

        let other = guild
            .members
            .iter()
            .any(|(id, m)| id != &member.user.id && m.roles.contains(&role.id));
        if !other {
            return Ok(role.clone());
        }
    }

    Err(UniqueColorError::NoColoredRole)
}

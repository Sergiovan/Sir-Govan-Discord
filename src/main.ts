import { Bot } from './bot/bot';
import { discord } from './secrets/secrets';

const bot = new Bot(discord.token, process.argv.includes('--beta'));
bot.connect();
import { readFileSync } from 'fs';
import nodeHtmlToImage from 'node-html-to-image';

export type TweetTheme = "dim" | "light" | "dark";

export type TweetMoreData = {
    avatar: string;
    name: string;
    verified: boolean;
    at: string;
    time: string; // xs, xm, xh or day mon year
    replyTo?: string;
    tweetText: string; // Raw html, encode beforehand
    replies: string;
    retweets: string;
    likes: string;
};

export type TweetData = {
    theme?: TweetTheme;
    retweeter: string;
    avatar: string;
    name: string;
    verified: boolean;
    at: string;
    replyTo?: string; 
    tweetText: string; // Raw html, encode beforehand
    image?: string;
    factCheck?: string;
    hour: string; // HH:MM PM/AM
    month: string; // Three letter month
    day: string; // Day of the month
    year: string;
    client: string;
    any_numbers: boolean; // If retweets, quotes or likes is more than 0
    retweets: string;
    quotes: string;
    likes: string;
    moreTweets: TweetMoreData[];
};

export async function createImage(data: TweetData) {
    const html = readFileSync('./html/tweet.hbs', "utf8"); 
    
    return await nodeHtmlToImage({
        html: html,
        content: data,
        transparent: true,
        puppeteerArgs: {
            defaultViewport: {
                width: 510,
                height: 10
            }
        },
        encoding: "binary"
    }) as Buffer;
}
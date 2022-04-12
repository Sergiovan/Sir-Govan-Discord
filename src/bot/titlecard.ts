import Ffmpeg = require("fluent-ffmpeg");
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'fs';
import { join } from 'path';
import { Logger } from "../utils";
import { Screenshotter } from "./screenshots";

const minimal_args = [
    '--autoplay-policy=user-gesture-required',
    '--disable-background-networking',
    '--disable-background-timer-throttling',
    '--disable-backgrounding-occluded-windows',
    '--disable-breakpad',
    '--disable-client-side-phishing-detection',
    '--disable-component-update',
    '--disable-default-apps',
    '--disable-dev-shm-usage',
    '--disable-domain-reliability',
    '--disable-extensions',
    '--disable-features=AudioServiceOutOfProcess',
    '--disable-hang-monitor',
    '--disable-ipc-flooding-protection',
    '--disable-notifications',
    '--disable-offer-store-unmasked-wallet-cards',
    '--disable-popup-blocking',
    '--disable-print-preview',
    '--disable-prompt-on-repost',
    '--disable-renderer-backgrounding',
    '--disable-setuid-sandbox',
    '--disable-speech-api',
    '--disable-sync',
    '--hide-scrollbars',
    '--ignore-gpu-blacklist',
    '--metrics-recording-only',
    '--mute-audio',
    '--no-default-browser-check',
    '--no-first-run',
    '--no-pings',
    '--no-sandbox',
    '--no-zygote',
    '--disable-gpu',
    '--password-store=basic',
    '--use-gl=swiftshader',
    '--use-mock-keychain',
];

let html: string | null = null;

export async function make_titlecard(episode_name: string, show_name: string, song_file: string, titlecard_name: string = 'titlecard') {
    Logger.time_start('titlecard_setup');
    const folder = mkdtempSync('/tmp/titlecard');
    let res: Buffer;
    try {
        if (html === null) {
            html = readFileSync('./html/titlecard.hbs', "utf8"); 
        }

        const episode_image = `${join(folder, 'episode.png')}`;
        const title_image = `${join(folder, 'title.png')}`;

        const episode_video = `${join(folder, 'episode.mp4')}`;
        const title_video = `${join(folder, 'title.mp4')}`;

        const output_files = `file '${episode_video}'\nfile '${title_video}'`;

        const output_files_file = `ffmpeg-concat-files.txt`; // On top level because bleh
        const final_output = `${join(folder, `${titlecard_name}.mp4`)}`;

        await (await Screenshotter.get()).screenshot(html, [{
            text: episode_name,
            output: episode_image
        },
        {
            text: show_name,
            output: title_image
        }]);

        Logger.time_end('titlecard_setup')

        // libx264

        Logger.time_start('titlecard_render1');
        await new Promise((res, rej) => Ffmpeg()
            .on('end', (val) => res(val))
            .on('error', (err, stdout, stderr) => { 
                console.error(err, stdout, stderr); 
                return rej(err); 
            }) 
            .input(episode_image).inputFormat('image2').loop()
            .videoCodec('libx264').outputOption('-pix_fmt yuv420p').fps(1/3).duration(3)
            .saveToFile(episode_video));
        Logger.time_end('titlecard_render1');

        Logger.time_start('titlecard_render2');
        await new Promise((res, rej) => Ffmpeg()
            .on('end', (val) => res(val))
            .on('error', (err, stdout, stderr) => { 
                console.error(err, stdout, stderr); 
                return rej(err); 
            }) 
            .input(title_image).inputFormat('image2').loop()
            .videoCodec('libx264').outputOption('-pix_fmt yuv420p').fps(1/4).duration(4)
            .saveToFile(title_video));
        Logger.time_end('titlecard_render2');

        writeFileSync(output_files_file, output_files);

        Logger.time_start('titlecard_render3');
        await new Promise((res, rej) => Ffmpeg()
            .on('end', (val) => res(val))
            .on('error', (err, stdout, stderr) => { 
                console.error(err, stdout, stderr); 
                return rej(err); 
            }) 
            .input(output_files_file).inputFormat('concat').inputOption('-safe 0')
            .input(song_file)
            .outputOptions([
                '-c:v libx264', '-crf 23', '-profile:v baseline', '-level 3.0', '-pix_fmt yuv420p',
                '-c:a aac', '-ac 2', '-b:a 128k',
                '-movflags faststart',
            ]).fps(1).saveToFile(final_output));

        Logger.time_end('titlecard_render3');

        res = readFileSync(final_output);

    } finally {
        rmSync(folder, { recursive: true });
    }

    return res;
}
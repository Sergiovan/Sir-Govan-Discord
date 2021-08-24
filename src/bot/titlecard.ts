import Ffmpeg = require("fluent-ffmpeg");
import nodeHtmlToImage from 'node-html-to-image';
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'fs';
import { join } from 'path';

export async function make_titlecard(episode_name: string, show_name: string, song_file: string, titlecard_name: string = 'titlecard') {
    const folder = mkdtempSync('/tmp/titlecard');
    let res: Buffer;
    try {
        const html = readFileSync('./html/titlecard.hbs', "utf8"); 

        const episode_image = `${join(folder, 'episode.png')}`;
        const title_image = `${join(folder, 'title.png')}`;

        const episode_video = `${join(folder, 'episode.mp4')}`;
        const title_video = `${join(folder, 'title.mp4')}`;

        const output_files = `file '${episode_video}'\nfile '${title_video}'`;

        const output_files_file = `ffmpeg-concat-files.txt`; // On top level because bleh
        const final_output = `${join(folder, `${titlecard_name}.mp4`)}`;

        await nodeHtmlToImage({
            html: html,
            puppeteerArgs: {
                defaultViewport: {
                    width: 1920,
                    height: 1080
                }
            },
            content: [{
                text: episode_name,
                output: episode_image
            },
            {
                text: show_name,
                output: title_image
            }],
        });

        await new Promise((res, rej) => Ffmpeg()
            .on('end', (val) => res(val))
            .on('error', (err) => { console.error(err); return rej(err); })
            .input(episode_image).inputFormat('image2').loop()
            .videoCodec('libx264').duration(3)
            .saveToFile(episode_video));

        await new Promise((res, rej) => Ffmpeg()
            .on('end', (val) => res(val))
            .on('error', (err) => { console.error(err); return rej(err); })
            .input(title_image).inputFormat('image2').loop()
            .videoCodec('libx264').duration(4)
            .saveToFile(title_video));

        writeFileSync(output_files_file, output_files);

        await new Promise((res, rej) => Ffmpeg()
            .on('end', (val) => res(val))
            .on('error', (err) => { console.error(err); return rej(err); })    
            .input(output_files_file).inputFormat('concat').inputOption('-safe 0')
            .input(song_file)
            .outputOptions([
                '-c:v libx264', '-crf 23', '-profile:v baseline', '-level 3.0', '-pix_fmt yuv420p',
                '-c:a aac', '-ac 2', '-b:a 128k',
                '-movflags faststart',
            ]).saveToFile(final_output));

        res = readFileSync(final_output);

    } finally {
        rmSync(folder, { recursive: true });
    }

    return res;
}
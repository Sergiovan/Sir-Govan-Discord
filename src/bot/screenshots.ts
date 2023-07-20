import { PuppeteerLifeCycleEvent } from 'puppeteer';
import { Cluster } from 'puppeteer-cluster';
import { compile } from 'handlebars';

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

type Content = { output: string; selector?: string; } & any;
type ViewportSize = { width: number, height: number };
type JobData = {
    html: string, 
    content: any,
    selector: string,
    output: string,
    transparent: boolean,
    waitUntil: PuppeteerLifeCycleEvent,
    min_size?: ViewportSize
}

// Singleton class to screenshot stuff because
// puppeteer takes a fuckload of time to start running
export class Screenshotter {

    static #instance: Screenshotter | null = null;
    cluster!: Cluster<JobData, Buffer>;

    static async get() {
        if (Screenshotter.#instance === null) {
            Screenshotter.#instance = new Screenshotter();
            await Screenshotter.#instance.#init();
        }
        return Screenshotter.#instance;
    }

    async #init() {
        this.cluster = await Cluster.launch({
            concurrency: Cluster.CONCURRENCY_CONTEXT,
            maxConcurrency: 4,
            puppeteerOptions: { 
                defaultViewport: {
                    width: 1920,
                    height: 1080
                }, 
                headless: "new",
                args: minimal_args
            },
        });

        this.cluster.task(async ({page, data}) => {
            const template = compile(data.html);
            const new_html = template(data.content);

            if (data.min_size) {
                await page.setViewport(data.min_size);
            }

            await page.setContent(new_html, { waitUntil: data.waitUntil });
            const elem = await page.$(data.selector);
            if (!elem) {
                throw Error(`No element matches selector: ${data.selector}`);
            }


            const buffer = await elem.screenshot({
                path: data.output, 
                type: 'png',
                omitBackground: data.transparent
            });

            return buffer as Buffer;
        });

    }

    async #deinit() {
        await this.cluster.idle();
        await this.cluster.close();
    }

    static async deinit() {
        if (Screenshotter.#instance === null) {
            return;
        }
        await Screenshotter.#instance.#deinit();
    }

    async screenshot(html: string, content: Content[], transparent: boolean = false, 
        waitUntil: PuppeteerLifeCycleEvent = 'domcontentloaded', min_size?: ViewportSize) {
        const screenshots: Array<Buffer> = await Promise.all(
            content.map((content) => {
                const {output, selector, ...otherContent} = content;
                return this.cluster.execute({
                    html,
                    content: otherContent,
                    selector: selector ?? 'body',
                    output, 
                    transparent,
                    waitUntil,
                    min_size
                });
            })
        );
        await this.cluster.idle();

        return screenshots;
    }

};
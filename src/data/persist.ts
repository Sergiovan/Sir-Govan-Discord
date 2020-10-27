import Storage from 'node-persist';

export class Persist {
    location: string;
    initialized: boolean = false;
    storage: Storage.LocalStorage;

    constructor(location: string) {
        this.location = location;
        this.storage = Storage.create({dir: this.location});
    }

    async init() {
        if (this.initialized) return;

        await this.storage.init();
        this.initialized = true;
    }

    async get(key: string, def: any = null) {
        return (await this.storage.getItem(key)) ?? def; 
    }

    async set(key: string, value: any) {
        return this.storage.setItem(key, value); 
    }
}
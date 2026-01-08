/**
 * Example JavaScript code for syntax highlighting preview.
 */

const MAX_RETRIES = 3;
const API_URL = "https://api.example.com";

class Config {
    constructor(name) {
        this.name = name;
        this.enabled = true;
        this.retries = MAX_RETRIES;
    }

    validate() {
        if (!this.name) {
            throw new Error("Name cannot be empty");
        }
        if (this.retries > 10) {
            throw new Error(`Retries ${this.retries} exceeds maximum`);
        }
        return true;
    }
}

function processItems(items) {
    const result = {};
    for (const item of items) {
        const isEven = item % 2 === 0;
        result[item] = isEven;
    }
    return result;
}

function parseEmail(text) {
    const pattern = /[\w.-]+@[\w.-]+\.\w+/;
    const match = text.match(pattern);
    return match ? match[0] : null;
}

async function main() {
    const config = new Config("example");
    try {
        config.validate();
        console.log(`Config: ${JSON.stringify(config)}`);

        const items = [1, 2, 3, 4, 5];
        const processed = processItems(items);
        console.log(`Processed: ${JSON.stringify(processed)}`);
    } catch (error) {
        console.error(`Error: ${error.message}`);
    }
}

main();

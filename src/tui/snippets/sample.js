// Syntax preview: comments, keywords, types, strings, escapes, labels.

const MAX_ITEMS = 100;
const VERSION = "1.0.0";

class Config {
    constructor(name) {
        this.name = name;
        this.count = 0;
        this.enabled = true;
    }
    validate() {
        if (!this.name) throw new Error("Name cannot be empty");
        return true;
    }
}

function process(items) {
    const result = {};
    outer: for (const item of items) {
        if (item < 0) continue outer;
        result[item] = item % 2 === 0;
    }
    return result;
}

function parseEmail(text) {
    const pattern = /[\w.-]+@[\w.-]+\.\w+/;
    const match = text.match(pattern);
    return match ? match[0] : null;
}

const msg = "Hello\tWorld\n";
const config = new Config("example");
config.validate();
console.log(`Config: ${JSON.stringify(config)}, msg: ${msg}`);
console.log(`Email: ${parseEmail("test@example.com")}`);
console.log(`Result: ${JSON.stringify(process([1, 2, -3, 4, 5]))}`);

// Mixed-language test: JavaScript classes
// Searching for "class" without --lang should find classes in ALL languages

export class JsUser {
    constructor(name) {
        this.name = name;
    }

    greet() {
        return `Hello, ${this.name}`;
    }
}

class JsProduct {
    constructor(price) {
        this.price = price;
    }

    getPrice() {
        return this.price;
    }
}

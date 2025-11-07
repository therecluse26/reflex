// Mixed-language test: TypeScript classes
// Searching for "class" without --lang should find classes in ALL languages

export class TsUser {
    constructor(public name: string) {}

    greet(): string {
        return `Hello, ${this.name}`;
    }
}

class TsProduct {
    constructor(private price: number) {}

    getPrice(): number {
        return this.price;
    }
}

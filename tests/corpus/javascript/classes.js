//! Test Corpus: JavaScript Classes (ES6)
//!
//! Expected symbols: 8+ classes
//!
//! Edge cases tested:
//! - Class declarations
//! - Inheritance
//! - Static methods
//! - Getters/setters
//! - Private fields

export class Point {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }

    distance() {
        return Math.sqrt(this.x ** 2 + this.y ** 2);
    }

    static origin() {
        return new Point(0, 0);
    }
}

class Person {
    #name; // Private field

    constructor(name, age) {
        this.#name = name;
        this.age = age;
    }

    getName() {
        return this.#name;
    }

    get formattedName() {
        return this.#name.toUpperCase();
    }

    set name(value) {
        this.#name = value;
    }
}

export class Employee extends Person {
    constructor(name, age, salary) {
        super(name, age);
        this.salary = salary;
    }

    getSalary() {
        return this.salary;
    }
}

class Counter {
    #count = 0;

    increment() {
        this.#count++;
    }

    get count() {
        return this.#count;
    }
}

export class Utils {
    static formatNumber(n) {
        return n.toFixed(2);
    }

    static parseNumber(s) {
        return parseFloat(s);
    }
}

class AsyncClass {
    async fetchData() {
        return fetch('/api/data').then(r => r.json());
    }

    async processData(url) {
        const data = await this.fetchData();
        return data;
    }
}

export class Container {
    #value;

    constructor(value) {
        this.#value = value;
    }

    map(fn) {
        return new Container(fn(this.#value));
    }

    flatMap(fn) {
        return fn(this.#value);
    }

    getValue() {
        return this.#value;
    }
}

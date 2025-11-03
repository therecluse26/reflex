//! Test Corpus: TypeScript Classes
//!
//! Expected symbols: 15+ classes and methods
//!
//! Edge cases tested:
//! - Class declarations
//! - Inheritance
//! - Abstract classes
//! - Private/protected/public members
//! - Static methods
//! - Getters/setters

export class Point {
    constructor(public x: number, public y: number) {}

    distance(): number {
        return Math.sqrt(this.x * this.x + this.y * this.y);
    }
}

class Person {
    private name: string;
    protected age: number;

    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }

    getName(): string {
        return this.name;
    }
}

export class Employee extends Person {
    private salary: number;

    constructor(name: string, age: number, salary: number) {
        super(name, age);
        this.salary = salary;
    }

    getSalary(): number {
        return this.salary;
    }
}

abstract class Shape {
    abstract area(): number;
    abstract perimeter(): number;
}

export class Rectangle extends Shape {
    constructor(private width: number, private height: number) {
        super();
    }

    area(): number {
        return this.width * this.height;
    }

    perimeter(): number {
        return 2 * (this.width + this.height);
    }
}

class Utils {
    static formatNumber(n: number): string {
        return n.toFixed(2);
    }

    static parseNumber(s: string): number {
        return parseFloat(s);
    }
}

export class Counter {
    private _count: number = 0;

    get count(): number {
        return this._count;
    }

    set count(value: number) {
        if (value >= 0) {
            this._count = value;
        }
    }

    increment(): void {
        this._count++;
    }
}

class GenericContainer<T> {
    private value: T;

    constructor(value: T) {
        this.value = value;
    }

    getValue(): T {
        return this.value;
    }

    setValue(value: T): void {
        this.value = value;
    }
}

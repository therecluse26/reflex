//! Test Corpus: TypeScript Generics
//!
//! Expected symbols: 12+ generic functions/classes
//!
//! Edge cases tested:
//! - Generic functions
//! - Generic classes
//! - Generic constraints
//! - Multiple type parameters
//! - Default type parameters

export function identity<T>(value: T): T {
    return value;
}

function pair<T, U>(first: T, second: U): [T, U] {
    return [first, second];
}

export function map<T, U>(arr: T[], fn: (item: T) => U): U[] {
    return arr.map(fn);
}

function withConstraint<T extends { length: number }>(value: T): number {
    return value.length;
}

export class Box<T> {
    constructor(private value: T) {}

    getValue(): T {
        return this.value;
    }

    setValue(value: T): void {
        this.value = value;
    }
}

class Pair<T, U> {
    constructor(public first: T, public second: U) {}

    swap(): Pair<U, T> {
        return new Pair(this.second, this.first);
    }
}

export function multipleConstraints<T extends string, U extends number>(
    a: T,
    b: U
): string {
    return `${a}: ${b}`;
}

function withDefault<T = string>(value: T): T {
    return value;
}

export interface GenericInterface<T, U = string> {
    first: T;
    second: U;
}

function complexGeneric<T extends { id: number }, U extends keyof T>(
    obj: T,
    key: U
): T[U] {
    return obj[key];
}

export function arrayMethods<T>(arr: T[]): {
    first: () => T | undefined;
    last: () => T | undefined;
} {
    return {
        first: () => arr[0],
        last: () => arr[arr.length - 1],
    };
}

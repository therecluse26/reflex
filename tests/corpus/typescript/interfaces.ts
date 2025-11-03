//! Test Corpus: TypeScript Interfaces
//!
//! Expected symbols: 12+ interfaces
//!
//! Edge cases tested:
//! - Interface declarations
//! - Extends
//! - Optional properties
//! - Readonly properties
//! - Index signatures
//! - Function types

export interface User {
    id: number;
    name: string;
    email: string;
}

interface ExtendedUser extends User {
    role: string;
    permissions: string[];
}

export interface Optional {
    required: string;
    optional?: number;
    nullable: string | null;
}

interface ReadonlyProps {
    readonly id: number;
    readonly createdAt: Date;
}

export interface StringMap {
    [key: string]: string;
}

interface NumberArray {
    [index: number]: number;
}

export interface Callback {
    (value: string): void;
}

interface GenericInterface<T> {
    value: T;
    getValue(): T;
}

export interface ComplexInterface<T, U> {
    first: T;
    second: U;
    combine(a: T, b: U): string;
}

interface WithMethods {
    method1(): void;
    method2(param: string): number;
    method3<T>(param: T): T;
}

export interface Nested {
    outer: {
        inner: {
            value: number;
        };
    };
}

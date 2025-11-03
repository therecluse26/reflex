//! Test Corpus: TypeScript Enums
//!
//! Expected symbols: 8 enums
//!
//! Edge cases tested:
//! - Numeric enums
//! - String enums
//! - Const enums
//! - Computed enum members

export enum Direction {
    North,
    South,
    East,
    West,
}

enum Status {
    Pending = 0,
    Active = 1,
    Completed = 2,
    Failed = -1,
}

export enum Color {
    Red = "RED",
    Green = "GREEN",
    Blue = "BLUE",
}

const enum ConstEnum {
    First = 1,
    Second = 2,
    Third = 3,
}

export enum FileAccess {
    None = 0,
    Read = 1 << 0,
    Write = 1 << 1,
    ReadWrite = Read | Write,
}

enum Mixed {
    No = 0,
    Yes = "YES",
}

export enum ComputedEnum {
    A = 1,
    B = A * 2,
    C = B * 2,
}

enum WithMethods {
    Value1,
    Value2,
}

export namespace WithMethods {
    export function toString(value: WithMethods): string {
        return value === WithMethods.Value1 ? "Value1" : "Value2";
    }
}

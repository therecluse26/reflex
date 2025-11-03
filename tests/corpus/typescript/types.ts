//! Test Corpus: TypeScript Type Aliases
//!
//! Expected symbols: 15+ type definitions
//!
//! Edge cases tested:
//! - Type aliases
//! - Union types
//! - Intersection types
//! - Conditional types
//! - Mapped types
//! - Utility types

export type ID = string | number;

type Status = 'pending' | 'active' | 'completed' | 'failed';

export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

type Point = { x: number; y: number };

export type Point3D = Point & { z: number };

type Nullable<T> = T | null;

export type DeepReadonly<T> = {
    readonly [P in keyof T]: DeepReadonly<T[P]>;
};

type ExtractString<T> = T extends string ? T : never;

export type ReturnType2<T> = T extends (...args: any[]) => infer R ? R : any;

type Partial2<T> = {
    [P in keyof T]?: T[P];
};

export type Required2<T> = {
    [P in keyof T]-?: T[P];
};

type Exclude2<T, U> = T extends U ? never : T;

export type Extract2<T, U> = T extends U ? T : never;

type FunctionType = (a: number, b: string) => boolean;

export type TupleType = [string, number, boolean];

type RecordType = Record<string, number>;

export type Awaited2<T> = T extends Promise<infer U> ? U : T;

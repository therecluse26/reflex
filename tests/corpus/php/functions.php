<?php
//! Test Corpus: PHP Functions
//!
//! Expected symbols: 15+ functions
//!
//! Edge cases tested:
//! - Function declarations
//! - Type hints
//! - Return types
//! - Default parameters
//! - Variadic functions

function simpleFunction(): void {
    echo "Hello";
}

function withParameters(string $name, int $age): string {
    return "$name is $age years old";
}

function withReturnType(int $a, int $b): int {
    return $a + $b;
}

function withDefaults(string $greeting = "Hello", string $name = "World"): string {
    return "$greeting, $name!";
}

function variadicFunction(string ...$args): array {
    return $args;
}

function nullable(?string $value): ?string {
    return $value;
}

function unionTypes(string|int $value): string {
    return (string) $value;
}

function referenceParameter(int &$value): void {
    $value *= 2;
}

function returnsArray(): array {
    return [1, 2, 3];
}

function anonymousFunction(): callable {
    return function(int $x): int {
        return $x * 2;
    };
}

function arrowFunction(): callable {
    $multiplier = 2;
    return fn($x) => $x * $multiplier;
}

function generatorFunction(): Generator {
    yield 1;
    yield 2;
    yield 3;
}

function mixedType(mixed $value): mixed {
    return $value;
}

function neverReturns(): never {
    throw new Exception("Never returns");
}

function voidFunction(): void {
    echo "Void";
}

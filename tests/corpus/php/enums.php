<?php
//! Test Corpus: PHP Enums (8.1+)
//!
//! Expected symbols: 6 enums
//!
//! Edge cases tested:
//! - Pure enums
//! - Backed enums (string/int)
//! - Enum methods

enum Status {
    case Pending;
    case Active;
    case Completed;
    case Failed;
}

enum StatusInt: int {
    case Pending = 0;
    case Active = 1;
    case Completed = 2;
    case Failed = -1;
}

enum Color: string {
    case Red = 'red';
    case Green = 'green';
    case Blue = 'blue';

    public function label(): string {
        return match($this) {
            self::Red => 'Red Color',
            self::Green => 'Green Color',
            self::Blue => 'Blue Color',
        };
    }
}

enum Permission: string {
    case Read = 'read';
    case Write = 'write';
    case Execute = 'execute';

    public static function fromString(string $value): ?self {
        return self::tryFrom($value);
    }
}

enum HttpMethod: string {
    case GET = 'GET';
    case POST = 'POST';
    case PUT = 'PUT';
    case DELETE = 'DELETE';
    case PATCH = 'PATCH';
}

enum Priority: int {
    case Low = 1;
    case Medium = 5;
    case High = 10;

    public function isHigh(): bool {
        return $this === self::High;
    }
}

<?php
//! Test Corpus: PHP Namespaces
//!
//! Expected symbols: 6+ namespaces
//!
//! Edge cases tested:
//! - Namespace declarations
//! - Nested namespaces
//! - Use statements

namespace App\Models;

class User {
    public function __construct(public string $name) {}
}

class Post {
    public function __construct(public string $title) {}
}

namespace App\Controllers;

class UserController {
    public function index(): void {
        echo "User index";
    }
}

namespace App\Services;

class EmailService {
    public function send(string $to, string $subject): void {
        echo "Sending email to $to";
    }
}

namespace Utils;

function helper(string $value): string {
    return strtoupper($value);
}

class StringUtils {
    public static function format(string $s): string {
        return trim($s);
    }
}

namespace {
    // Global namespace
    function globalFunction(): void {
        echo "Global";
    }
}

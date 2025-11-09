<?php
//! Test Corpus: PHP Classes
//!
//! Expected symbols: 10+ classes with various features
//!
//! Edge cases tested:
//! - Class declarations
//! - Abstract classes
//! - Final classes
//! - Visibility modifiers
//! - Static methods

class Point {
    public float $x;
    public float $y;

    public function __construct(float $x, float $y) {
        $this->x = $x;
        $this->y = $y;
    }

    public function distance(): float {
        return sqrt($this->x ** 2 + $this->y ** 2);
    }
}

class Person {
    private string $name;
    protected int $age;

    public function __construct(string $name, int $age) {
        $this->name = $name;
        $this->age = $age;
    }

    public function getName(): string {
        return $this->name;
    }
}

final class Employee extends Person {
    private float $salary;

    public function __construct(string $name, int $age, float $salary) {
        parent::__construct($name, $age);
        $this->salary = $salary;
    }

    public function getSalary(): float {
        return $this->salary;
    }
}

abstract class Shape {
    abstract public function area(): float;
    abstract public function perimeter(): float;
}

class Rectangle extends Shape {
    public function __construct(private float $width, private float $height) {}

    public function area(): float {
        return $this->width * $this->height;
    }

    public function perimeter(): float {
        return 2 * ($this->width + $this->height);
    }
}

class Utils {
    public static function formatNumber(float $n): string {
        return number_format($n, 2);
    }

    public static function parseNumber(string $s): float {
        return floatval($s);
    }
}

class Counter {
    private int $count = 0;

    public function increment(): void {
        $this->count++;
    }

    public function getCount(): int {
        return $this->count;
    }
}

// Edge case: Class implementing multiple interfaces
interface Authenticatable {
    public function authenticate(): bool;
}

interface HasPermissions {
    public function hasPermission(string $permission): bool;
}

interface JWTSubject {
    public function getJWTIdentifier();
}

class AdminUser implements Authenticatable, HasPermissions {
    private string $username;

    public function __construct(string $username) {
        $this->username = $username;
    }

    public function authenticate(): bool {
        return true;
    }

    public function hasPermission(string $permission): bool {
        return true;
    }
}

/**
 * Complex edge case: Class with large docblock, extends base class, implements multiple interfaces
 *
 * @property string $name
 * @property string $email
 * @property-read int $id
 * @property-read string $created_at
 * @property-read Collection|Role[] $roles
 * @property-read Collection|Permission[] $permissions
 * @property-read Workflow $workflow
 * @property-read Collection|NotificationSetting[] $notificationSettings
 * @property-read Collection|Watch[] $watches
 *
 **/
class ComplexUser extends AdminUser implements HasPermissions, JWTSubject
{
    private string $email;
    private int $userId;

    public function __construct(string $username, string $email) {
        parent::__construct($username);
        $this->email = $email;
    }

    public function hasPermission(string $permission): bool {
        return parent::hasPermission($permission);
    }

    public function getJWTIdentifier() {
        return $this->userId;
    }
}

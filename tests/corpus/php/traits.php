<?php
//! Test Corpus: PHP Traits
//!
//! Expected symbols: 6+ traits
//!
//! Edge cases tested:
//! - Trait definitions
//! - Trait usage
//! - Trait conflict resolution

trait Loggable {
    public function log(string $message): void {
        echo "[LOG] " . $message . PHP_EOL;
    }

    protected function debug(string $message): void {
        echo "[DEBUG] " . $message . PHP_EOL;
    }
}

trait Timestampable {
    private int $createdAt;
    private int $updatedAt;

    public function setCreatedAt(): void {
        $this->createdAt = time();
    }

    public function setUpdatedAt(): void {
        $this->updatedAt = time();
    }

    public function getCreatedAt(): int {
        return $this->createdAt;
    }
}

trait Serializable {
    public function toJson(): string {
        return json_encode($this);
    }

    public function toArray(): array {
        return (array) $this;
    }
}

class User {
    use Loggable, Timestampable, Serializable;

    public function __construct(private string $name) {
        $this->setCreatedAt();
    }
}

trait A {
    public function conflictMethod(): string {
        return "A";
    }
}

trait B {
    public function conflictMethod(): string {
        return "B";
    }
}

class ConflictResolution {
    use A, B {
        A::conflictMethod insteadof B;
        B::conflictMethod as conflictMethodB;
    }
}

trait RequiresMethods {
    abstract public function requiredMethod(): void;

    public function callRequired(): void {
        $this->requiredMethod();
    }
}

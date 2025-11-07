<?php
// Mixed-language test: PHP classes
// Searching for "class" without --lang should find classes in ALL languages

class PhpUser {
    private $name;

    public function __construct($name) {
        $this->name = $name;
    }
}

class PhpProduct {
    private $price;

    public function getPrice() {
        return $this->price;
    }
}

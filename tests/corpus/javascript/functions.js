//! Test Corpus: JavaScript Functions
//!
//! Expected symbols: 15+ functions
//!
//! Edge cases tested:
//! - Function declarations
//! - Arrow functions
//! - Anonymous functions
//! - Generator functions
//! - Async functions

export function regularFunction() {
    console.log('Regular function');
}

function privateFunction() {
    return 42;
}

export const arrowFunction = () => {
    console.log('Arrow');
};

const arrowWithParam = (x) => x * 2;

export const arrowWithBlock = (x, y) => {
    const result = x + y;
    return result;
};

function higherOrderFunction(callback) {
    return callback(42);
}

export function* generatorFunction() {
    yield 1;
    yield 2;
    yield 3;
}

async function asyncFunction() {
    return Promise.resolve('async');
}

export const asyncArrow = async () => {
    const result = await asyncFunction();
    return result;
};

function functionWithDefaults(a = 10, b = 20) {
    return a + b;
}

export const destructuringParams = ({ x, y }) => {
    return x + y;
};

function restParams(...args) {
    return args.reduce((acc, val) => acc + val, 0);
}

export const iife = (function() {
    console.log('IIFE');
    return 42;
})();

const objectMethod = {
    method() {
        console.log('Object method');
    },
    async asyncMethod() {
        return 'async method';
    },
};

export function* asyncGeneratorFunction() {
    yield Promise.resolve(1);
    yield Promise.resolve(2);
}

const curried = (a) => (b) => (c) => a + b + c;

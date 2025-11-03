//! Test Corpus: JavaScript Arrow Functions
//!
//! Expected symbols: 15+ arrow functions
//!
//! Edge cases tested:
//! - Various arrow function syntaxes
//! - Implicit returns
//! - Parentheses variations
//! - Async arrows

export const simple = () => 42;

const withParam = (x) => x * 2;

export const multiParam = (x, y) => x + y;

const noParens = x => x + 1;

export const withBlock = (x) => {
    const result = x * 2;
    return result;
};

const returningObject = () => ({ key: 'value' });

export const asyncArrow = async () => {
    return await Promise.resolve(42);
};

const asyncWithAwait = async (url) => {
    const response = await fetch(url);
    return response.json();
};

export const curried = (a) => (b) => (c) => a + b + c;

const higherOrder = (fn) => (x) => fn(x);

export const mapFunction = (arr) => arr.map(x => x * 2);

const filterMap = (arr) => arr.filter(x => x > 0).map(x => x * 2);

export const destructuring = ({ x, y }) => x + y;

const restParams = (...args) => args.reduce((a, b) => a + b, 0);

export const defaultParams = (x = 10, y = 20) => x + y;

const nestedArrow = () => () => () => 42;

export const immediateInvoke = (() => {
    console.log('Invoked');
    return 42;
})();

const arrayMethods = [1, 2, 3].map(x => x * 2).filter(x => x > 2);

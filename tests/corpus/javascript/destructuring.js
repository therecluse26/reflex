//! Test Corpus: JavaScript Destructuring
//!
//! This file tests destructuring patterns in various contexts

export function objectDestructuring({ name, age }) {
    console.log(name, age);
}

const { x, y } = { x: 1, y: 2 };

export const { a, b: renamed } = { a: 10, b: 20 };

function arrayDestructuring([first, second]) {
    return first + second;
}

export const [one, two, three] = [1, 2, 3];

const { nested: { value } } = { nested: { value: 42 } };

export function defaultValues({ name = 'Unknown', age = 0 } = {}) {
    console.log(name, age);
}

const { x: xVal, ...rest } = { x: 1, y: 2, z: 3 };

export const [head, ...tail] = [1, 2, 3, 4, 5];

function swapValues() {
    let a = 1, b = 2;
    [a, b] = [b, a];
    return [a, b];
}

export const {
    prop1,
    prop2: {
        nested1,
        nested2
    }
} = {
    prop1: 'value1',
    prop2: {
        nested1: 'nested1',
        nested2: 'nested2'
    }
};

const functionWithDestructuring = ({
    required,
    optional = 'default',
    ...others
}) => {
    console.log(required, optional, others);
};

export function parameterDestructuring([a, b], { x, y }) {
    return a + b + x + y;
}

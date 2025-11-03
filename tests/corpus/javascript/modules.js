//! Test Corpus: JavaScript Modules
//!
//! This file tests import/export patterns
//!
//! Edge cases tested:
//! - Named exports
//! - Default exports
//! - Re-exports
//! - Import aliases

export const VERSION = '1.0.0';

export function helper() {
    return 'helper';
}

export class Config {
    constructor() {
        this.debug = false;
    }
}

const privateValue = 42;

export { privateValue as publicValue };

export default function mainFunction() {
    console.log('Main');
}

export const namedExport1 = 'value1';
export const namedExport2 = 'value2';

function internalFunction() {
    return 'internal';
}

export { internalFunction };

// Re-export from another module
export { something } from './other';
export * from './utilities';

const obj = {
    method1() {},
    method2() {},
};

export const { method1, method2 } = obj;

export async function asyncExport() {
    return 'async export';
}

export const arrowExport = () => 'arrow';

export class ExportedClass {
    method() {
        return 'method';
    }
}

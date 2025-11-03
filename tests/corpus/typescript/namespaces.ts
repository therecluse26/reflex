//! Test Corpus: TypeScript Namespaces
//!
//! Expected symbols: 6+ namespaces
//!
//! Edge cases tested:
//! - Namespace declarations
//! - Nested namespaces
//! - Namespace merging

export namespace Utils {
    export function format(s: string): string {
        return s.toUpperCase();
    }

    export function parse(s: string): number {
        return parseInt(s, 10);
    }
}

namespace Internal {
    export class Helper {
        static doSomething(): void {
            console.log("Internal");
        }
    }
}

export namespace Shapes {
    export namespace Circle {
        export function area(radius: number): number {
            return Math.PI * radius * radius;
        }
    }

    export namespace Rectangle {
        export function area(width: number, height: number): number {
            return width * height;
        }
    }
}

namespace Validators {
    export interface StringValidator {
        isValid(s: string): boolean;
    }

    export class EmailValidator implements StringValidator {
        isValid(s: string): boolean {
            return s.includes("@");
        }
    }
}

// Namespace merging
namespace Validators {
    export class URLValidator implements StringValidator {
        isValid(s: string): boolean {
            return s.startsWith("http");
        }
    }
}

export namespace Config {
    export const VERSION = "1.0.0";
    export const DEBUG = false;

    export function getConfig(): object {
        return { VERSION, DEBUG };
    }
}

//! Test Corpus: TypeScript Decorators
//!
//! Expected symbols: classes and methods with decorators
//!
//! Edge cases tested:
//! - Class decorators
//! - Method decorators
//! - Property decorators
//! - Parameter decorators

function sealed(constructor: Function) {
    Object.seal(constructor);
    Object.seal(constructor.prototype);
}

function logged(target: any, key: string, descriptor: PropertyDescriptor) {
    const original = descriptor.value;
    descriptor.value = function (...args: any[]) {
        console.log(`Calling ${key}`);
        return original.apply(this, args);
    };
}

function readonly(target: any, key: string) {
    Object.defineProperty(target, key, {
        writable: false,
    });
}

function validate(target: any, key: string, index: number) {
    console.log(`Validate parameter ${index} of ${key}`);
}

@sealed
export class Component {
    @readonly
    name: string = "Component";

    @logged
    render(): void {
        console.log("Rendering");
    }

    process(@validate data: string): void {
        console.log(data);
    }
}

@sealed
class Service {
    @logged
    async fetch(url: string): Promise<string> {
        return "data";
    }

    @logged
    @readonly
    static VERSION = "1.0.0";
}

export function Component2(config: { name: string }) {
    return function <T extends { new (...args: any[]): {} }>(constructor: T) {
        return class extends constructor {
            name = config.name;
        };
    };
}

@Component2({ name: "MyComponent" })
class MyComponent {
    render() {
        console.log("Rendering MyComponent");
    }
}

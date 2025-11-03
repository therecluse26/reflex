//! Test Corpus: TypeScript Async/Await
//!
//! Expected symbols: 12+ async functions
//!
//! Edge cases tested:
//! - async functions
//! - Promise usage
//! - await expressions
//! - Promise.all, Promise.race
//! - Error handling with try/catch

export async function fetchData(): Promise<string> {
    return "data";
}

async function fetchUser(id: number): Promise<{ id: number; name: string }> {
    return { id, name: "User" };
}

export async function processData(): Promise<void> {
    const data = await fetchData();
    console.log(data);
}

async function multipleAwaits(): Promise<number> {
    const user = await fetchUser(1);
    const data = await fetchData();
    return user.id;
}

export async function errorHandling(): Promise<string> {
    try {
        const result = await fetchData();
        return result;
    } catch (error) {
        return "error";
    }
}

async function promiseAll(): Promise<string[]> {
    const results = await Promise.all([
        fetchData(),
        fetchData(),
        fetchData(),
    ]);
    return results;
}

export async function promiseRace(): Promise<string> {
    return await Promise.race([fetchData(), fetchData()]);
}

function returnsPromise(): Promise<number> {
    return Promise.resolve(42);
}

export async function asyncArrow(): Promise<void> {
    const fn = async () => {
        return "arrow";
    };
    await fn();
}

async function* asyncGenerator(): AsyncGenerator<number> {
    yield 1;
    yield 2;
    yield 3;
}

export async function consumeAsyncGen(): Promise<void> {
    for await (const num of asyncGenerator()) {
        console.log(num);
    }
}

async function promiseChain(): Promise<string> {
    return fetchData()
        .then((data) => data.toUpperCase())
        .then((upper) => `Result: ${upper}`);
}

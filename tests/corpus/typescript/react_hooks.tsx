//! Test Corpus: React Hooks (TSX)
//!
//! Expected symbols: 10+ React components and hooks
//!
//! Edge cases tested:
//! - Function components
//! - useState, useEffect, useMemo, useCallback
//! - Custom hooks
//! - JSX syntax

import React, { useState, useEffect, useMemo, useCallback } from 'react';

export function Counter() {
    const [count, setCount] = useState(0);

    return (
        <div>
            <p>Count: {count}</p>
            <button onClick={() => setCount(count + 1)}>Increment</button>
        </div>
    );
}

function UserProfile({ userId }: { userId: number }) {
    const [user, setUser] = useState<{ name: string } | null>(null);

    useEffect(() => {
        fetch(`/api/users/${userId}`)
            .then(res => res.json())
            .then(setUser);
    }, [userId]);

    return <div>{user?.name}</div>;
}

export function ExpensiveComponent({ data }: { data: number[] }) {
    const sorted = useMemo(() => {
        console.log('Sorting...');
        return [...data].sort((a, b) => a - b);
    }, [data]);

    return <div>{sorted.join(', ')}</div>;
}

function CallbackExample() {
    const [count, setCount] = useState(0);

    const handleClick = useCallback(() => {
        setCount(c => c + 1);
    }, []);

    return <button onClick={handleClick}>Click</button>;
}

export function useCounter(initialValue: number = 0) {
    const [count, setCount] = useState(initialValue);

    const increment = useCallback(() => {
        setCount(c => c + 1);
    }, []);

    const decrement = useCallback(() => {
        setCount(c => c - 1);
    }, []);

    const reset = useCallback(() => {
        setCount(initialValue);
    }, [initialValue]);

    return { count, increment, decrement, reset };
}

function useLocalStorage(key: string, initialValue: string) {
    const [value, setValue] = useState(() => {
        return localStorage.getItem(key) || initialValue;
    });

    useEffect(() => {
        localStorage.setItem(key, value);
    }, [key, value]);

    return [value, setValue] as const;
}

export const MemoizedComponent = React.memo(function MemoizedComponent({ name }: { name: string }) {
    return <div>Hello, {name}</div>;
});

function ComplexComponent() {
    const [isOpen, setIsOpen] = useState(false);
    const [data, setData] = useState<string[]>([]);

    useEffect(() => {
        if (isOpen) {
            fetch('/api/data')
                .then(res => res.json())
                .then(setData);
        }
    }, [isOpen]);

    return (
        <div>
            <button onClick={() => setIsOpen(!isOpen)}>Toggle</button>
            {isOpen && <ul>{data.map(item => <li key={item}>{item}</li>)}</ul>}
        </div>
    );
}

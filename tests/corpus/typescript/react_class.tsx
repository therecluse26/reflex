//! Test Corpus: React Class Components (TSX)
//!
//! Expected symbols: 6+ class components
//!
//! Edge cases tested:
//! - Class components
//! - Lifecycle methods
//! - State management
//! - Props and state types

import React, { Component } from 'react';

interface CounterProps {
    initialCount: number;
}

interface CounterState {
    count: number;
}

export class CounterClass extends Component<CounterProps, CounterState> {
    constructor(props: CounterProps) {
        super(props);
        this.state = { count: props.initialCount };
    }

    increment = () => {
        this.setState({ count: this.state.count + 1 });
    };

    render() {
        return (
            <div>
                <p>Count: {this.state.count}</p>
                <button onClick={this.increment}>Increment</button>
            </div>
        );
    }
}

class LifecycleComponent extends Component {
    componentDidMount() {
        console.log('Mounted');
    }

    componentDidUpdate(prevProps: any) {
        console.log('Updated');
    }

    componentWillUnmount() {
        console.log('Unmounting');
    }

    render() {
        return <div>Lifecycle</div>;
    }
}

export class FetchComponent extends Component<{ url: string }, { data: any }> {
    state = { data: null };

    async componentDidMount() {
        const response = await fetch(this.props.url);
        const data = await response.json();
        this.setState({ data });
    }

    render() {
        return <div>{JSON.stringify(this.state.data)}</div>;
    }
}

class FormComponent extends Component<{}, { value: string }> {
    state = { value: '' };

    handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.setState({ value: e.target.value });
    };

    handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        console.log('Submitted:', this.state.value);
    };

    render() {
        return (
            <form onSubmit={this.handleSubmit}>
                <input value={this.state.value} onChange={this.handleChange} />
                <button type="submit">Submit</button>
            </form>
        );
    }
}

export class ErrorBoundary extends Component<{ children: React.ReactNode }, { hasError: boolean }> {
    state = { hasError: false };

    static getDerivedStateFromError(error: Error) {
        return { hasError: true };
    }

    componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
        console.error('Error:', error, errorInfo);
    }

    render() {
        if (this.state.hasError) {
            return <div>Something went wrong</div>;
        }
        return this.props.children;
    }
}

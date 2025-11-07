"""Test Corpus: Python Functions

Expected symbols: 15 functions
- 2 regular functions (greet, add)
- 2 async functions (fetch_data, async_generator)
- 1 generator function (count_up)
- 1 nested function (outer contains inner)
- 2 decorated functions (cached, timed)
- 2 lambda functions (square, multiply)
- 1 function with type hints (calculate)
- 1 function with default args (create_user)
- 1 function with *args/**kwargs (flexible)
- 1 function with decorators chained (double_decorated)
- 1 recursive function (factorial)

Edge cases tested:
- Function declarations
- Async functions
- Generator functions
- Nested functions
- Decorators
- Lambda functions
- Type hints
- Default arguments
- Variable arguments (*args, **kwargs)
- Recursion
"""

import asyncio
from typing import List, Dict, Optional
from functools import wraps


def greet(name: str) -> str:
    """Regular function with type hints"""
    return f"Hello, {name}!"


def add(a: int, b: int) -> int:
    """Simple addition function"""
    return a + b


async def fetch_data(url: str) -> Dict:
    """Async function"""
    await asyncio.sleep(0.1)
    return {"url": url, "data": "response"}


def count_up(limit: int):
    """Generator function"""
    current = 0
    while current < limit:
        yield current
        current += 1


async def async_generator(n: int):
    """Async generator"""
    for i in range(n):
        await asyncio.sleep(0.01)
        yield i


def outer(x: int):
    """Function with nested function"""
    def inner(y: int):
        """Nested function"""
        return x + y
    return inner


def cache_decorator(func):
    """Decorator function"""
    cache = {}
    @wraps(func)
    def wrapper(*args):
        if args not in cache:
            cache[args] = func(*args)
        return cache[args]
    return wrapper


@cache_decorator
def cached(n: int) -> int:
    """Function with caching decorator"""
    return n * n


def timer_decorator(func):
    """Timer decorator"""
    @wraps(func)
    def wrapper(*args, **kwargs):
        result = func(*args, **kwargs)
        return result
    return wrapper


@timer_decorator
def timed(duration: float):
    """Function with timer decorator"""
    return f"Running for {duration}s"


# Lambda functions (assigned to variables)
square = lambda x: x * x
multiply = lambda x, y: x * y


def calculate(values: List[float], operation: str = "sum") -> float:
    """Function with type hints and default args"""
    if operation == "sum":
        return sum(values)
    elif operation == "avg":
        return sum(values) / len(values)
    return 0.0


def create_user(
    name: str,
    email: str,
    age: int = 0,
    active: bool = True
) -> Dict[str, any]:
    """Function with default arguments"""
    return {
        "name": name,
        "email": email,
        "age": age,
        "active": active
    }


def flexible(*args, **kwargs) -> Dict:
    """Function with *args and **kwargs"""
    return {
        "args": args,
        "kwargs": kwargs
    }


@cache_decorator
@timer_decorator
def double_decorated(x: int, y: int) -> int:
    """Function with multiple decorators"""
    return x + y


def factorial(n: int) -> int:
    """Recursive function"""
    if n <= 1:
        return 1
    return n * factorial(n - 1)


# Class methods are tested in classes.py
class Helper:
    """Helper class for testing methods"""

    @staticmethod
    def static_method() -> str:
        """Static method"""
        return "static"

    @classmethod
    def class_method(cls) -> str:
        """Class method"""
        return "class"

    def instance_method(self) -> str:
        """Instance method"""
        return "instance"

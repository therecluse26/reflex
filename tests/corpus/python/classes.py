"""Test Corpus: Python Classes

Expected symbols: 10 classes
- 2 regular classes (User, Product)
- 1 inherited class (Employee)
- 1 abstract class (Shape)
- 1 concrete implementation (Circle)
- 1 dataclass (Config)
- 1 nested class (Container.Inner)
- 1 class with decorators (Singleton)
- 1 multiple inheritance (DatabaseModel)
- 1 class with metaclass (Entity)

Edge cases tested:
- Class declarations
- Inheritance (single and multiple)
- Abstract base classes
- Dataclasses
- Nested classes
- Class decorators
- Metaclasses
- Type hints
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Optional, List


class User:
    """Regular class with __init__"""
    def __init__(self, name: str, email: str):
        self.name = name
        self.email = email

    def greet(self) -> str:
        return f"Hello, {self.name}"


class Product:
    """Class with class variables"""
    category: str = "general"

    def __init__(self, name: str, price: float):
        self.name = name
        self.price = price


class Employee(User):
    """Inherited class"""
    def __init__(self, name: str, email: str, employee_id: int):
        super().__init__(name, email)
        self.employee_id = employee_id

    def get_id(self) -> int:
        return self.employee_id


class Shape(ABC):
    """Abstract base class"""
    @abstractmethod
    def area(self) -> float:
        pass

    @abstractmethod
    def perimeter(self) -> float:
        pass


class Circle(Shape):
    """Concrete implementation of abstract class"""
    def __init__(self, radius: float):
        self.radius = radius

    def area(self) -> float:
        return 3.14159 * self.radius ** 2

    def perimeter(self) -> float:
        return 2 * 3.14159 * self.radius


@dataclass
class Config:
    """Dataclass with type hints"""
    host: str
    port: int
    timeout: float = 30.0
    debug: bool = False


class Container:
    """Class with nested inner class"""
    def __init__(self, value: int):
        self.value = value

    class Inner:
        """Nested class"""
        def __init__(self, data: str):
            self.data = data


def singleton(cls):
    """Decorator for singleton pattern"""
    instances = {}
    def wrapper(*args, **kwargs):
        if cls not in instances:
            instances[cls] = cls(*args, **kwargs)
        return instances[cls]
    return wrapper


@singleton
class Singleton:
    """Class with decorator"""
    def __init__(self):
        self.value = 0


class Mixin:
    """Mixin class for multiple inheritance"""
    def log(self, message: str):
        print(f"LOG: {message}")


class DatabaseModel(Mixin, User):
    """Multiple inheritance"""
    def __init__(self, name: str, email: str, db_id: int):
        super().__init__(name, email)
        self.db_id = db_id

    def save(self):
        self.log(f"Saving {self.name}")


class EntityMeta(type):
    """Metaclass"""
    def __new__(mcs, name, bases, attrs):
        attrs['entity_type'] = 'auto'
        return super().__new__(mcs, name, bases, attrs)


class Entity(metaclass=EntityMeta):
    """Class with metaclass"""
    def __init__(self, id: int):
        self.id = id

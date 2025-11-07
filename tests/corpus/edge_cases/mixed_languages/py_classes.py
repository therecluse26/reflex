"""Mixed-language test: Python classes
Searching for 'class' without --lang should find classes in ALL languages
"""

class PyUser:
    def __init__(self, name: str):
        self.name = name

    def greet(self) -> str:
        return f"Hello, {self.name}"


class PyProduct:
    def __init__(self, price: float):
        self.price = price

    def get_price(self) -> float:
        return self.price

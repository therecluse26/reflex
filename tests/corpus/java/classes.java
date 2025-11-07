// Test Corpus: Java Classes
//
// Expected symbols: 10 classes
// - 1 public class (Point)
// - 1 class with private fields (Person)
// - 1 abstract class (Shape)
// - 1 concrete class extending abstract (Circle)
// - 1 final class (Constants)
// - 1 static nested class (Container.Node)
// - 1 inner class (Outer.Inner)
// - 1 anonymous class (in method)
// - 1 class with generics (Box)
// - 1 enum class (Status)
//
// Edge cases tested:
// - Public classes
// - Access modifiers (public, private, protected)
// - Abstract classes
// - Final classes
// - Static nested classes
// - Inner classes
// - Anonymous classes
// - Generic classes
// - Enums

package corpus;

import java.util.ArrayList;
import java.util.List;

// Public class
public class Point {
    public double x;
    public double y;

    public Point(double x, double y) {
        this.x = x;
        this.y = y;
    }

    public double distance() {
        return Math.sqrt(x * x + y * y);
    }
}

// Class with private fields
class Person {
    private String name;
    private int age;

    public Person(String name, int age) {
        this.name = name;
        this.age = age;
    }

    public String getName() {
        return name;
    }

    public int getAge() {
        return age;
    }
}

// Abstract class
abstract class Shape {
    protected String color;

    public abstract double area();
    public abstract double perimeter();

    public String getColor() {
        return color;
    }
}

// Concrete class extending abstract
class Circle extends Shape {
    private double radius;

    public Circle(double radius) {
        this.radius = radius;
        this.color = "red";
    }

    @Override
    public double area() {
        return Math.PI * radius * radius;
    }

    @Override
    public double perimeter() {
        return 2 * Math.PI * radius;
    }
}

// Final class (cannot be extended)
final class Constants {
    public static final double PI = 3.14159;
    public static final String VERSION = "1.0.0";

    private Constants() {
        // private constructor
    }
}

// Class with static nested class
class Container {
    private List<Node> nodes;

    public Container() {
        this.nodes = new ArrayList<>();
    }

    // Static nested class
    public static class Node {
        private String value;

        public Node(String value) {
            this.value = value;
        }

        public String getValue() {
            return value;
        }
    }
}

// Class with inner class
class Outer {
    private int outerValue;

    public Outer(int value) {
        this.outerValue = value;
    }

    // Inner class (non-static)
    public class Inner {
        private int innerValue;

        public Inner(int value) {
            this.innerValue = value;
        }

        public int sum() {
            return outerValue + innerValue;
        }
    }
}

// Generic class
class Box<T> {
    private T value;

    public Box(T value) {
        this.value = value;
    }

    public T getValue() {
        return value;
    }

    public void setValue(T value) {
        this.value = value;
    }
}

// Enum
enum Status {
    PENDING,
    ACTIVE,
    COMPLETED,
    CANCELLED;

    public boolean isFinished() {
        return this == COMPLETED || this == CANCELLED;
    }
}

// Example usage with anonymous class
class Examples {
    public void anonymousExample() {
        // Anonymous class implementing Runnable
        Runnable task = new Runnable() {
            @Override
            public void run() {
                System.out.println("Anonymous class");
            }
        };
        task.run();
    }

    public void genericExample() {
        Box<String> stringBox = new Box<>("Hello");
        Box<Integer> intBox = new Box<>(42);
    }
}

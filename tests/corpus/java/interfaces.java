// Test Corpus: Java Interfaces
//
// Expected symbols: 8 interfaces
// - 1 basic interface (Drawable)
// - 1 interface extending another (Resizable extends Drawable)
// - 1 interface with default methods (Logger)
// - 1 functional interface (Calculator)
// - 1 interface with static methods (MathUtils)
// - 1 interface with constants (Constants)
// - 1 generic interface (Repository)
// - 1 marker interface (Serializable)
//
// Edge cases tested:
// - Basic interface declarations
// - Interface inheritance (extends)
// - Default methods (Java 8+)
// - Static methods in interfaces
// - Functional interfaces (@FunctionalInterface)
// - Interface constants
// - Generic interfaces
// - Marker interfaces (empty)

package corpus;

// Basic interface
public interface Drawable {
    void draw();
    void setColor(String color);
}

// Interface extending another
interface Resizable extends Drawable {
    void resize(double scale);
    double getWidth();
    double getHeight();
}

// Interface with default methods
interface Logger {
    void log(String message);

    default void info(String message) {
        log("INFO: " + message);
    }

    default void error(String message) {
        log("ERROR: " + message);
    }

    default void debug(String message) {
        log("DEBUG: " + message);
    }
}

// Functional interface (single abstract method)
@FunctionalInterface
interface Calculator {
    int calculate(int a, int b);

    // Default methods are allowed
    default int add(int a, int b) {
        return a + b;
    }
}

// Interface with static methods
interface MathUtils {
    static int max(int a, int b) {
        return (a > b) ? a : b;
    }

    static int min(int a, int b) {
        return (a < b) ? a : b;
    }

    static double average(int... numbers) {
        if (numbers.length == 0) return 0;
        int sum = 0;
        for (int n : numbers) {
            sum += n;
        }
        return (double) sum / numbers.length;
    }
}

// Interface with constants
interface Constants {
    int MAX_SIZE = 100;
    String DEFAULT_NAME = "Unknown";
    double PI = 3.14159;
}

// Generic interface
interface Repository<T> {
    T findById(int id);
    void save(T entity);
    void delete(T entity);
    java.util.List<T> findAll();
}

// Marker interface (empty interface for type checking)
interface Serializable {
    // No methods - just a marker
}

// Example implementations
class Circle implements Drawable {
    private String color;

    @Override
    public void draw() {
        System.out.println("Drawing circle");
    }

    @Override
    public void setColor(String color) {
        this.color = color;
    }
}

class Rectangle implements Resizable {
    private double width;
    private double height;
    private String color;

    @Override
    public void draw() {
        System.out.println("Drawing rectangle");
    }

    @Override
    public void setColor(String color) {
        this.color = color;
    }

    @Override
    public void resize(double scale) {
        this.width *= scale;
        this.height *= scale;
    }

    @Override
    public double getWidth() {
        return width;
    }

    @Override
    public double getHeight() {
        return height;
    }
}

class ConsoleLogger implements Logger {
    @Override
    public void log(String message) {
        System.out.println(message);
    }
}

class UserRepository implements Repository<String> {
    @Override
    public String findById(int id) {
        return "User" + id;
    }

    @Override
    public void save(String entity) {
        System.out.println("Saving: " + entity);
    }

    @Override
    public void delete(String entity) {
        System.out.println("Deleting: " + entity);
    }

    @Override
    public java.util.List<String> findAll() {
        return new java.util.ArrayList<>();
    }
}

// Mixed-language test: Java classes
// Searching for "class" without --lang should find classes in ALL languages

package mixed;

public class JavaUser {
    private String name;

    public JavaUser(String name) {
        this.name = name;
    }

    public String greet() {
        return "Hello, " + name;
    }
}

class JavaProduct {
    private double price;

    public JavaProduct(double price) {
        this.price = price;
    }

    public double getPrice() {
        return price;
    }
}

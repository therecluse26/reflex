// Test Corpus: Go Functions
//
// Expected symbols: 12 functions
// - 3 regular functions (Add, Greet, Calculate)
// - 2 methods (see structs.go: Counter.Increment, Counter.Value)
// - 1 variadic function (Sum)
// - 1 function with multiple returns (Divide)
// - 1 function with named returns (Stats)
// - 1 closure (outer returns closure)
// - 1 init function (init)
// - 1 defer example (DeferExample)
// - 1 goroutine function (AsyncWork)
//
// Edge cases tested:
// - Function declarations
// - Methods (pointer and value receivers)
// - Variadic functions
// - Multiple return values
// - Named return values
// - Closures
// - init functions
// - defer statements
// - goroutines

package corpus

import (
	"fmt"
	"time"
)

// Add is a simple function
func Add(a, b int) int {
	return a + b
}

// Greet returns a greeting
func Greet(name string) string {
	return fmt.Sprintf("Hello, %s!", name)
}

// Calculate performs a calculation
func Calculate(x, y float64, op string) float64 {
	switch op {
	case "add":
		return x + y
	case "sub":
		return x - y
	case "mul":
		return x * y
	case "div":
		if y != 0 {
			return x / y
		}
		return 0
	default:
		return 0
	}
}

// Sum is a variadic function
func Sum(numbers ...int) int {
	total := 0
	for _, n := range numbers {
		total += n
	}
	return total
}

// Divide returns result and error (multiple returns)
func Divide(a, b float64) (float64, error) {
	if b == 0 {
		return 0, fmt.Errorf("division by zero")
	}
	return a / b, nil
}

// Stats returns named return values
func Stats(numbers []int) (min, max, avg int) {
	if len(numbers) == 0 {
		return 0, 0, 0
	}

	min = numbers[0]
	max = numbers[0]
	sum := 0

	for _, n := range numbers {
		if n < min {
			min = n
		}
		if n > max {
			max = n
		}
		sum += n
	}

	avg = sum / len(numbers)
	return // naked return with named values
}

// OuterFunction returns a closure
func OuterFunction(x int) func(int) int {
	return func(y int) int {
		return x + y
	}
}

// init is a special initialization function
func init() {
	fmt.Println("Package initialized")
}

// DeferExample demonstrates defer
func DeferExample() {
	defer fmt.Println("deferred")
	fmt.Println("regular")
}

// AsyncWork runs in a goroutine
func AsyncWork(id int, done chan bool) {
	time.Sleep(100 * time.Millisecond)
	fmt.Printf("Worker %d done\n", id)
	done <- true
}

// HigherOrder takes a function as parameter
func HigherOrder(fn func(int) int, value int) int {
	return fn(value)
}

// Example of using functions
func ExampleUsage() {
	// Regular function call
	sum := Add(5, 10)

	// Variadic function
	total := Sum(1, 2, 3, 4, 5)

	// Multiple returns
	result, err := Divide(10, 2)
	if err != nil {
		fmt.Println("Error:", err)
	}

	// Named returns
	min, max, avg := Stats([]int{1, 5, 3, 9, 2})

	// Closure
	addFive := OuterFunction(5)
	value := addFive(10) // returns 15

	// Defer
	DeferExample()

	// Goroutine
	done := make(chan bool)
	go AsyncWork(1, done)
	<-done

	// Use variables
	_ = sum
	_ = total
	_ = result
	_ = min
	_ = max
	_ = avg
	_ = value
}

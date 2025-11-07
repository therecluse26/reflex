// Test Corpus: Go Structs
//
// Expected symbols: 8 structs
// - 3 regular structs (Point, Person, Config)
// - 1 empty struct (Empty)
// - 1 struct with embedded fields (Employee)
// - 1 struct with tags (User)
// - 1 anonymous struct (in function)
// - 1 struct with methods (Counter)
//
// Edge cases tested:
// - Named fields
// - Empty structs
// - Embedded fields
// - Struct tags (JSON, DB)
// - Pointer receivers
// - Value receivers
// - Anonymous structs

package corpus

import "encoding/json"

// Point is a basic struct
type Point struct {
	X float64
	Y float64
}

// Person with various field types
type Person struct {
	Name  string
	Age   int
	Email *string // pointer field
}

// Config with nested struct
type Config struct {
	Host string
	Port int
	TLS  struct {
		Enabled bool
		CertPath string
	}
}

// Empty is an empty struct
type Empty struct{}

// Employee embeds Person
type Employee struct {
	Person // embedded field
	EmployeeID int
	Department string
}

// User with struct tags
type User struct {
	ID       int    `json:"id" db:"user_id"`
	Username string `json:"username" db:"username"`
	Email    string `json:"email" db:"email"`
	IsActive bool   `json:"is_active" db:"is_active"`
}

// Counter with methods
type Counter struct {
	value int
}

// Increment is a pointer receiver method
func (c *Counter) Increment() {
	c.value++
}

// Value is a value receiver method
func (c Counter) Value() int {
	return c.value
}

// Function that uses anonymous struct
func CreateAnonymous() interface{} {
	return struct {
		Name string
		Age  int
	}{
		Name: "Anonymous",
		Age:  0,
	}
}

// Example of struct initialization
func ExampleUsage() {
	// Regular initialization
	p := Point{X: 1.0, Y: 2.0}

	// Pointer initialization
	person := &Person{
		Name: "Alice",
		Age:  30,
	}

	// Embedded struct
	emp := Employee{
		Person: Person{
			Name: "Bob",
			Age:  35,
		},
		EmployeeID: 12345,
		Department: "Engineering",
	}

	// Use the structs
	_ = p
	_ = person
	_ = emp
}

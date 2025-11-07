// Test Corpus: C Structs
//
// Expected symbols: 10 structs
// - 2 basic structs (point, person)
// - 2 typedef structs (Vector, Config)
// - 1 nested struct (container with inner struct)
// - 1 anonymous struct (in typedef)
// - 1 struct with bit fields (Flags)
// - 1 struct with union (Data)
// - 1 forward declaration (Node)
// - 1 struct with function pointers (Operations)
//
// Edge cases tested:
// - Basic struct declarations
// - typedef struct
// - Nested structs
// - Anonymous structs
// - Bit fields
// - Structs with unions
// - Forward declarations
// - Function pointers in structs
// - Self-referential structs (linked lists)

#include <stdio.h>
#include <stdlib.h>

// Basic struct
struct point {
    double x;
    double y;
};

// Basic struct with more fields
struct person {
    char name[50];
    int age;
    double height;
};

// typedef struct (named)
typedef struct vector {
    double x;
    double y;
    double z;
} Vector;

// typedef struct (typedef only, no struct name)
typedef struct {
    char host[256];
    int port;
    int timeout;
} Config;

// Nested struct
struct container {
    int id;
    struct inner {
        int value;
        char label[20];
    } data;
};

// Anonymous struct in typedef
typedef struct {
    int hours;
    int minutes;
    int seconds;
} Time;

// Struct with bit fields
struct flags {
    unsigned int is_active : 1;
    unsigned int is_admin : 1;
    unsigned int permissions : 4;
    unsigned int reserved : 2;
};

// Struct with union
struct data {
    enum { INT_TYPE, FLOAT_TYPE, STRING_TYPE } type;
    union {
        int i;
        float f;
        char *s;
    } value;
};

// Forward declaration and self-referential struct
struct node;

struct node {
    int value;
    struct node *next;
    struct node *prev;
};

typedef struct node Node;

// Struct with function pointers
struct operations {
    int (*add)(int, int);
    int (*subtract)(int, int);
    int (*multiply)(int, int);
};

// Helper functions
int add_impl(int a, int b) {
    return a + b;
}

int subtract_impl(int a, int b) {
    return a - b;
}

int multiply_impl(int a, int b) {
    return a * b;
}

// Example usage
void example_usage() {
    // Basic struct
    struct point p1 = {1.0, 2.0};

    // typedef struct
    Vector v1 = {1.0, 2.0, 3.0};

    // Anonymous struct via typedef
    Config cfg = {"localhost", 8080, 30};

    // Nested struct
    struct container c = {
        .id = 1,
        .data = {
            .value = 42,
            .label = "test"
        }
    };

    // Bit fields
    struct flags f = {
        .is_active = 1,
        .is_admin = 0,
        .permissions = 7,
        .reserved = 0
    };

    // Union struct
    struct data d;
    d.type = INT_TYPE;
    d.value.i = 42;

    // Linked list node
    Node *head = (Node*)malloc(sizeof(Node));
    head->value = 1;
    head->next = NULL;
    head->prev = NULL;

    // Function pointer struct
    struct operations ops = {
        .add = add_impl,
        .subtract = subtract_impl,
        .multiply = multiply_impl
    };

    int result = ops.add(5, 3);

    // Cleanup
    free(head);
}

// More examples with struct pointers
void struct_pointers() {
    struct person *p = (struct person*)malloc(sizeof(struct person));
    p->age = 30;
    free(p);
}

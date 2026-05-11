// ======================================================
// RUST PROGRAMMING LANGUAGE - COMPLETE DEMO FILE
// Run once to demonstrate all core concepts
// ======================================================

use std::collections::HashMap;
use std::fs;
// use std::sync::{Arc, Mutex};

// ======================================================
// VARIABLES & DATA TYPES
// ======================================================

fn demo_variables() {

    println!("\n========== VARIABLES & DATA TYPES ==========");

    let name = "Rust";
    let age = 20;

    let marks: i32 = 95;
    let price: f64 = 99.99;
    let grade: char = 'A';
    let is_passed: bool = true;

    println!("Name: {}", name);
    println!("Age: {}", age);
    println!("Marks: {}", marks);
    println!("Price: {}", price);
    println!("Grade: {}", grade);
    println!("Passed: {}", is_passed);
}

// ======================================================
// STRINGS
// ======================================================

fn demo_strings() {

    println!("\n========== STRINGS ==========");

    let str_slice: &str = "Rust";
    let mut string_obj = String::from("Hello");

    string_obj.push_str(" Rust");

    println!("&str: {}", str_slice);
    println!("String: {}", string_obj);
}

// ======================================================
// CONSTANTS
// ======================================================

const MAX_USERS: i32 = 100;

fn demo_constants() {

    println!("\n========== CONSTANTS ==========");

    println!("Max Users: {}", MAX_USERS);
}

// ======================================================
// OPERATORS
// ======================================================

fn demo_operators() {

    println!("\n========== OPERATORS ==========");

    let a = 10;
    let b = 5;

    println!("Add: {}", a + b);
    println!("Sub: {}", a - b);
    println!("Mul: {}", a * b);
    println!("Div: {}", a / b);

    println!("Equal: {}", a == b);
    println!("Greater: {}", a > b);

    let is_true = true;
    let is_ok = false;

    println!("AND: {}", is_true && !is_ok);
}

// ======================================================
// CONTROL FLOW
// ======================================================

fn demo_control_flow() {

    println!("\n========== CONTROL FLOW ==========");

    let age = 18;

    if age >= 18 {
        println!("Adult");
    } else {
        println!("Minor");
    }

    let num = 2;

    match num {
        1 => println!("One"),
        2 => println!("Two"),
        _ => println!("Other"),
    }
}

// ======================================================
// LOOPS
// ======================================================

fn demo_loops() {

    println!("\n========== LOOPS ==========");

    for i in 1..4 {
        println!("For loop: {}", i);
    }

    let mut i = 0;

    while i < 3 {
        println!("While loop: {}", i);
        i += 1;
    }
}

// ======================================================
// FUNCTIONS
// ======================================================

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn demo_functions() {

    println!("\n========== FUNCTIONS ==========");

    let result = add(10, 20);

    println!("Sum: {}", result);
}

// ======================================================
// ERROR HANDLING
// ======================================================

fn demo_error_handling() {

    println!("\n========== ERROR HANDLING ==========");

    let file = fs::read_to_string("demo.txt");

    match file {
        Ok(content) => println!("File Content: {}", content),
        Err(_) => println!("File not found (expected in demo)"),
    }
}

// ======================================================
// OWNERSHIP & BORROWING
// ======================================================

fn demo_ownership() {

    println!("\n========== OWNERSHIP ==========");

    let s1 = String::from("Rust");
    let s2 = s1; // ownership moved

    println!("Owned String: {}", s2);
}

fn demo_borrowing() {

    println!("\n========== BORROWING ==========");

    let s = String::from("Hello Rust");
    let r = &s;

    println!("Original: {}", s);
    println!("Borrowed: {}", r);
}

// ======================================================
// DATA STRUCTURES
// ======================================================

fn demo_data_structures() {

    println!("\n========== DATA STRUCTURES ==========");

    // Array
    let arr = [1, 2, 3];
    println!("Array: {}", arr[0]);

    // Vector
    let mut vec = vec![1, 2, 3];
    vec.push(4);
    println!("Vector: {:?}", vec);

    // Tuple
    let person = ("Ali", 22);
    println!("Tuple: {} {}", person.0, person.1);

    // HashMap
    let mut map = HashMap::new();
    map.insert("name", "Ali");
    println!("HashMap: {:?}", map);
}

// ======================================================
// STRUCTS
// ======================================================

struct Person {
    name: String,
    age: u32,
}

fn demo_structs() {

    println!("\n========== STRUCTS ==========");

    let p = Person {
        name: String::from("Ali"),
        age: 22,
    };

    println!("Name: {}", p.name);
    println!("Age: {}", p.age);
}

fn demo_expressions() {
    println!("\n========== RUST EXPRESSIONS ==========");

    // =========================
    // 1. BLOCK EXPRESSION
    // =========================
    let block_result = {
        let a = 10;
        let b = 20;
        a + b // returned from block
    };

    println!("Block expression result: {}", block_result);

    // =========================
    // 2. MATCH EXPRESSION
    // =========================
    let number = 2;

    let match_result = match number {
        1 => "One",
        2 => "Two",
        3 => "Three",
        _ => "Other",
    };

    println!("Match expression result: {}", match_result);

    // =========================
    // 3. LOOP EXPRESSION
    // =========================
    let mut i = 0;

    let loop_result = loop {
        i += 1;

        if i == 5 {
            break i * 10; // returning value from loop
        }
    };

    println!("Loop expression result: {}", loop_result);
}

// ======================================================
// MAIN FUNCTION
// ======================================================

fn main() {

    demo_variables();
    demo_strings();
    demo_constants();
    demo_operators();
    demo_control_flow();
    demo_loops();
    demo_functions();
    demo_error_handling();
    demo_ownership();
    demo_borrowing();
    demo_data_structures();
    demo_structs();
    demo_expressions();

    println!("\n========== END OF RUST DEMO ==========");
}
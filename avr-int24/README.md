# AVR-Int24

This library provides a 24-bit signed integer type, `Int24`, for Rust.
It is designed for use on AVR microcontrollers.

## Features

- 24-bit signed integer type (`Int24`)
- Saturating arithmetic operations: addition, subtraction, multiplication, division
- Bitwise operations: shift left, shift right
- Comparison operations
- Conversions to and from `i16` and `i32`

## Usage

To use the `Int24` type, add this library to your `Cargo.toml` and then use it in your code:

```rust
use avr_int24::Int24;

let a = Int24::from_i32(1000);
let b = Int24::from_i32(2000);

let c = a + b;

assert_eq!(c.to_i32(), 3000);
```
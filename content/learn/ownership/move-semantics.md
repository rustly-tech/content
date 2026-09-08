Every value in Rust has exactly one owner. When the owner goes out of scope, the
value is dropped. That single rule is what lets Rust free memory without a
garbage collector and without you writing `free`.

## Assignment moves

```rust
let a = String::from("hello");
let b = a;
```

In most languages you now have two names for one string, or two copies of it.
In Rust you have neither. `a` **moved** into `b`. There is still exactly one
`String`, still exactly one owner, and `a` is no longer usable.

The `String` itself is three words on the stack — a pointer to the text, a
length, and a capacity — and the text lives on the heap. The move copies those
three words into `b`. It does **not** copy the heap text; that would be
expensive, and Rust does not do expensive things quietly.

So why not leave `a` usable? Because then both `a` and `b` would point at the
same heap allocation, and both would try to free it when they went out of scope.
That is a double free. Rust's answer is not to add a runtime check but to make
the situation unrepresentable: after the move, using `a` is a compile error.

## Copy types are different

```rust
let a = 5;
let b = a;
println!("{a}");  // fine
```

An `i32` is just its bits. Copying it costs nothing, and there is no heap
allocation for two owners to fight over. Types like this implement the `Copy`
trait, and assignment copies instead of moving. Integers, floats, `bool`,
`char`, and tuples of `Copy` types are all `Copy`.

The rule of thumb: **if a type owns a heap allocation or any other resource, it
is not `Copy`.** `String` owns heap memory. `Vec<T>` owns heap memory. `File`
owns a file descriptor. None of them are `Copy`.

## Passing to a function moves too

```rust
fn consume(s: String) {}

let s = String::from("hi");
consume(s);
// s is gone
```

Passing by value is an assignment into the parameter, so the same rule applies.
This is not a special case to memorise; it falls out of the one rule.

## Reading the error

When you use a moved value, the compiler says this:

```text
error[E0382]: borrow of moved value: `a`
 --> src/main.rs:4:20
  |
2 |     let a = String::from("hello");
  |         - move occurs because `a` has type `String`, which does not implement the `Copy` trait
3 |     let b = a;
  |             - value moved here
4 |     println!("{a}");
  |                ^ value borrowed here after move
```

It is worth reading all of it rather than just the first line. It tells you
three separate things: **why** the move happened (`String` is not `Copy`),
**where** it happened (line 3), and **where** you tried to use the value
afterwards (line 4). Almost every ownership error you will meet has this shape,
and once you can read it you can fix the error without guessing.

## Three ways out

Once you understand the error, the fix is a design decision, not a trick.

1. **Borrow instead.** If the second user only needs to *look* at the value,
   take `&String` or `&str`. Nothing moves.
2. **Clone.** If you genuinely need two independent values, `a.clone()` says so
   out loud. It is not a defeat — it is a cost you have chosen and can see.
3. **Restructure.** Often the move is telling you that two pieces of code both
   want to own the same thing, and only one of them should.

Reach for borrowing first. Reach for `clone` when you mean it. Reaching for
`clone` every time the compiler complains works, and it will slowly turn your
program into something slower and harder to reason about than it needed to be.

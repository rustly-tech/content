You are given a program that does not compile.

`summarise` takes a `String` by value, so calling it moves the caller's string.
The next line tries to print that string, and the borrow checker stops you with
`E0382`.

## Your task

Make the program compile and produce the expected output, **without cloning and
without changing `main`'s output**.

Read the input as a single line of text. For each line of input, print:

```
<length> <first word>
```

where `<length>` is the number of bytes in the whole line and `<first word>` is
the first whitespace-separated word. Then print the original line again,
unchanged.

For the input `hello world`, the expected output is:

```
11 hello
hello world
```

## Constraints

- Do not use `.clone()`, `.to_owned()`, or `String::from` on the input.
- `main` must still print the original line after the summary.
- Input is one line, may contain multiple spaces, and always contains at least
  one word.

## Hint

The compiler is telling you that `summarise` takes ownership when it only needs
to read. Ask what the function actually requires, then give it exactly that.

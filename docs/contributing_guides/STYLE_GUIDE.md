# R3BL code style guide

<a id="markdown-r3bl-code-style-guide" name="r3bl-code-style-guide"></a>

<!-- TOC -->

- [General guidelines](#general-guidelines)
- [Naming conventions](#naming-conventions)
- [Formatting conventions](#formatting-conventions)
- [Commenting conventions](#commenting-conventions)
- [Programming practices](#programming-practices)
  - [Favor enums over booleans](#favor-enums-over-booleans)
- [Additional Tips](#additional-tips)

<!-- /TOC -->

## General guidelines

<a id="markdown-general-guidelines" name="general-guidelines"></a>

Set of conventions or standards for writing and designing code.

## Naming conventions

<a id="markdown-naming-conventions" name="naming-conventions"></a>

- Use a consistent naming convention for variables, functions, and modules.
- Use descriptive names that clearly indicate the purpose of the item.
- Use singular enum and struct names in Rust. This is in line with Rust's naming conventions for
  types, which generally favor singular names.
  - E.g. `enum FileStatus { ... }` instead of `enum FileStatuses { ... }`.
  - E.g. `struct File { ... }` instead of `struct Files { ... }`.
- Naming constants: Use all uppercase letters and underscores to separate words. E.g. `MAX_NUMBER`.

## Formatting conventions

<a id="markdown-formatting-conventions" name="formatting-conventions"></a>

- Use consistent indentation throughout your code.
- Use spaces, not tabs, for indentation.
- Each level of indentation should be 4 spaces.
- Use a consistent spacing style for operators and parentheses.
- Keep lines of code to a reasonable length.

## Commenting conventions

<a id="markdown-commenting-conventions" name="commenting-conventions"></a>

- Use comments to explain complex code or non-obvious logic.

## Programming practices

<a id="markdown-programming-practices" name="programming-practices"></a>

- Avoid global variables.
- Avoid hard-coded values. Use constants instead.

### Favor enums over booleans

<a id="markdown-favor-enums-over-booleans" name="favor-enums-over-booleans"></a>

1. **Enums are more expressive**: Boolean variables can only have two values, true and false. This
   can be limiting, especially when you are dealing with more than two possible states. For example,
   if you are tracking the status of a file, you might want to use an enum with values such as Open,
   Closed, Saved, and Unsaved. This makes it much clearer what the possible states of the file are,
   and it can also make the code more self-documenting.

2. **Enums are easier to match on**: When you are working with boolean variables, you often need to
   use a series of if-else statements to check the value of the variable and execute the appropriate
   code. This can make the code difficult to read and maintain. With enums, you can use a match
   expression to match on the value of the enum and execute the appropriate code. This can make the
   code more readable and easier to maintain.

3. **Enums are safer**: Boolean variables can be easily misused. For example, if you have a variable
   that is called isSaved, you might accidentally assign the value false to it when you mean to
   assign the value true. This can lead to bugs in your code. With enums, you are less likely to
   make these kinds of mistakes because the values of the enum are named.

4. **Enums are more future-proof**: If you are using boolean variables, and you later decide that
   you need more than two possible states, you will need to refactor your code. This can be
   time-consuming and error-prone. With enums, you can simply add a new value to the enum. This is
   much easier and less error-prone.

5. **Enums make code more self-documenting**: When you use enums, the possible states of a variable
   are explicitly declared in the code. This makes the code more self-documenting and easier to
   understand for other developers.

6. **Enums can improve compiler warnings**: The Rust compiler can provide more helpful warnings when
   you use enums instead of booleans. For example, if you try to use an enum value in a context
   where it is not expected, the compiler will warn you about this error.

In general, enums are a **more powerful and expressive** way to represent states than boolean
variables. They can make your code more readable, maintainable, safer, and future-proof.

Here is an example of how to use an enum to represent the status of a file:

```Rust
enum FileStatus {
    Open,
    Closed,
    Saved,
    Unsaved,
}
```

You can then use this enum to track the status of a \*\*file:

```Rust
l**et mut file = File::new();

file.open();

match file.status() {
    FileStatus::Open => println!("The file is open."),
    FileStatus::Closed => println!("The file is closed."),
    FileStatus::Saved => println!("The file is saved."),
    FileStatus::Unsaved => println!("The file is unsaved."),
}
```

## Additional Tips

<a id="markdown-additional-tips" name="additional-tips"></a>

- Use rustfmt to automatically format your code.
- Favor convention over ceremony.
- Favor readability over cleverness.
- Favor readability over brevity.
- Favor readability over verbosity.
- Favor well named intermediate variables over brevity.
- Favor loosely coupled and strongly coherent over tightly coupled.
- Favor shallow imports over deep ones by re-exporting from modules.

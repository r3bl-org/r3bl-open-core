# Advanced Markdown Testing Document 📚

@title: Complex Markdown Features Test
@tags: markdown, testing, complex, unicode
@authors: R3BL Team, Markdown Parser
@date: 2025-07-10

This document tests various complex markdown features that real-world documents contain.

## Unicode and Emoji Support 🌟

### Emojis in Different Contexts

Emojis can appear:
- At the beginning: 🚀 Space exploration
- In the middle: The rocket 🚀 launched successfully
- At the end: Mission accomplished 🎉

#### Mathematical Symbols

Mathematical expressions with Unicode: ∑, ∆, π, ∞, ≠, ≤, ≥

#### International Characters

- **Japanese**: こんにちは世界 (Hello World)
- **Arabic**: مرحبا بالعالم
- **Russian**: Привет мир
- **German**: Hallö Wörld with ümlauts

## Complex Lists and Nesting 📝

### Multi-level nested list with code blocks

1. **First level item**
   - Second level bullet
   - Another second level item
     ```python
     def hello_world():
         print("Hello from nested code!")
     ```
   - Back to second level
     1. Third level numbered
     2. Another third level
        - Fourth level bullet
        - More fourth level content
          ```rust
          fn main() {
              println!("Deeply nested code 🦀");
          }
          ```

2. **Complex list item with multiple elements**

   This list item contains:
   - Multiple paragraphs
   - Code blocks
   - Links and formatting

   Here's some code:
   ```typescript
   interface ComplexInterface {
       name: string;
       values: number[];
       callback: (data: any) => void;
   }
   ```

   And here's a [complex link](https://example.com/path?param=value&other=test#section "Link with title").

### Task lists with various states

- [x] ✅ Completed task
- [ ] 📋 Pending task
- [x] 🎯 Another completed task
- [ ] 🔄 Work in progress
  - [x] Sub-task completed
  - [ ] Sub-task pending
  - [x] Another sub-task ✨

## Code Blocks and Syntax Highlighting 💻

### Different languages

**Rust example:**
```rust
use std::collections::HashMap;

#[derive(Debug)]
struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: &str, age: u32) -> Self {
        Self {
            name: name.to_string(),
            age,
        }
    }

    fn greet(&self) -> String {
        format!("Hi, I'm {} and I'm {} years old! 👋", self.name, self.age)
    }
}

fn main() {
    let person = Person::new("Alice", 30);
    println!("{}", person.greet());
}
```

**JavaScript with complex features:**
```javascript
class AsyncDataProcessor {
    constructor(config) {
        this.config = { timeout: 5000, ...config };
        this.cache = new Map();
    }

    async processData(data) {
        const key = this.generateKey(data);

        if (this.cache.has(key)) {
            return this.cache.get(key);
        }

        try {
            const result = await this.performAsyncOperation(data);
            this.cache.set(key, result);
            return result;
        } catch (error) {
            console.error(`Processing failed: ${error.message} 🚨`);
            throw new ProcessingError(error.message);
        }
    }

    generateKey(data) {
        return btoa(JSON.stringify(data)).slice(0, 16);
    }
}
```

**SQL with complex query:**
```sql
WITH recursive_cte AS (
    SELECT
        id,
        name,
        parent_id,
        0 as level
    FROM categories
    WHERE parent_id IS NULL

    UNION ALL

    SELECT
        c.id,
        c.name,
        c.parent_id,
        r.level + 1
    FROM categories c
    INNER JOIN recursive_cte r ON c.parent_id = r.id
)
SELECT
    CONCAT(REPEAT('  ', level), name) as indented_name,
    level,
    COUNT(*) OVER (PARTITION BY level) as siblings
FROM recursive_cte
ORDER BY level, name;
```

## Complex Links and References 🔗

### Various link formats

1. Simple link: [GitHub](https://github.com)
2. Link with title: [GitHub Repository](https://github.com/r3bl-org/r3bl-open-core "R3BL Open Core Repository")
3. Reference link: [R3BL Website][r3bl-ref]
4. Complex URL: [API Endpoint](https://api.example.com/v2/users?filter=active&sort=name&page=1&limit=50#results)

### Image references

![Simple image](https://via.placeholder.com/300x200 "Placeholder image")

![Complex image with Unicode caption](https://via.placeholder.com/400x300 "Test image with emojis 🖼️📸")

## Tables with Complex Content 📊

| Feature | Status | Notes | Progress |
|---------|--------|-------|----------|
| **Basic parsing** | ✅ Complete | All basic markdown elements | 100% |
| *Unicode support* | 🔄 In Progress | Handling complex Unicode | 85% |
| `Code highlighting` | ✅ Complete | Multiple languages supported | 100% |
| [Link processing](https://example.com) | ⚠️ Partial | Some edge cases remain | 75% |
| Emoji rendering 🎨 | ✅ Complete | Full emoji support | 100% |

## Special Characters and Edge Cases 🔍

### Escape sequences

These should be escaped: \*not italic\*, \[not a link\], \`not code\`

### Mixed formatting

This text has **bold with *nested italic* inside** and `code with **bold inside**` and *italic with `code inside`*.

### Edge case combinations

- **Bold at start** of line
- Line ending with **bold at end**
- `Code at start` of line
- Line ending with `code at end`
- *Italic at start* of line
- Line ending with *italic at end*

## Blockquotes with Nesting 💬

> This is a simple blockquote.

> This is a complex blockquote with multiple paragraphs.
>
> It contains **formatting**, `code`, and [links](https://example.com).
>
> > This is a nested blockquote.
> >
> > > And this is double-nested!
> > >
> > > It even contains code:
> > > ```rust
> > > println!("Hello from nested quote! 📦");
> > > ```

## Horizontal Rules and Separators

---

Above and below this text are horizontal rules.

***

Different style of horizontal rule.

___

Yet another style.

---

## Final Section: Real-world Edge Cases 🧪

### Common markdown pitfalls

1. **Unclosed formatting**: This *should be italic but is unclosed
2. **Conflicting markers**: This **bold *and italic** conflict*
3. **URL-like text**: Not a link: https://example.com but this is: [actual link](https://example.com)
4. **Code-like text**: Not code: `unclosed backtick vs proper `code`

### Performance test content

This section contains content designed to test parser performance:

```
Long lines of text that exceed typical width limits and contain various Unicode characters like 🚀🌟💻📚🔍🧪 and international text like ñoño, résumé, naïve, café, piñata, jalapeño, and mathematical symbols ∑∆πα∞≠≤≥ to ensure the parser handles complex character sequences efficiently without performance degradation.
```

---

**Document Stats:**
- Lines: ~200+
- Characters: ~8000+
- Unicode: ✅ Extensive
- Complexity: 🔥 High

[r3bl-ref]: https://r3bl.com "R3BL LLC Official Website"

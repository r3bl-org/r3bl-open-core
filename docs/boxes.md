# Box-Drawing Character Reference

Provide a quick reference for Unicode box-drawing characters commonly used in ASCII diagrams.

## Light Box-Drawing Characters

### Lines
| Char | Unicode | Name |
|------|---------|------|
| `─` | U+2500 | HORIZONTAL |
| `│` | U+2502 | VERTICAL |

### Corners
| Char | Unicode | Name |
|------|---------|------|
| `┌` | U+250C | DOWN AND RIGHT (top-left) |
| `┐` | U+2510 | DOWN AND LEFT (top-right) |
| `└` | U+2514 | UP AND RIGHT (bottom-left) |
| `┘` | U+2518 | UP AND LEFT (bottom-right) |

### T-Junctions
| Char | Unicode | Name | Connects |
|------|---------|------|----------|
| `├` | U+251C | VERTICAL AND RIGHT | up, down, right |
| `┤` | U+2524 | VERTICAL AND LEFT | up, down, left |
| `┬` | U+252C | DOWN AND HORIZONTAL | left, right, down |
| `┴` | U+2534 | UP AND HORIZONTAL | left, right, up |

### Cross
| Char | Unicode | Name | Connects |
|------|---------|------|----------|
| `┼` | U+253C | VERTICAL AND HORIZONTAL | all four directions |

## Heavy Box-Drawing Characters

### Lines
| Char | Unicode | Name |
|------|---------|------|
| `━` | U+2501 | HEAVY HORIZONTAL |
| `┃` | U+2503 | HEAVY VERTICAL |

### Corners
| Char | Unicode | Name |
|------|---------|------|
| `┏` | U+250F | HEAVY DOWN AND RIGHT |
| `┓` | U+2513 | HEAVY DOWN AND LEFT |
| `┗` | U+2517 | HEAVY UP AND RIGHT |
| `┛` | U+251B | HEAVY UP AND LEFT |

## Double-Line Box-Drawing Characters

### Lines
| Char | Unicode | Name |
|------|---------|------|
| `═` | U+2550 | DOUBLE HORIZONTAL |
| `║` | U+2551 | DOUBLE VERTICAL |

### Corners
| Char | Unicode | Name |
|------|---------|------|
| `╔` | U+2554 | DOUBLE DOWN AND RIGHT |
| `╗` | U+2557 | DOUBLE DOWN AND LEFT |
| `╚` | U+255A | DOUBLE UP AND RIGHT |
| `╝` | U+255D | DOUBLE UP AND LEFT |

## Rounded Corners (Light Arc)

| Char | Unicode | Name |
|------|---------|------|
| `╭` | U+256D | ARC DOWN AND RIGHT |
| `╮` | U+256E | ARC DOWN AND LEFT |
| `╯` | U+256F | ARC UP AND LEFT |
| `╰` | U+2570 | ARC UP AND RIGHT |

## Quick Copy-Paste Box Templates

### Simple Box
```
┌─────────┐
│ content │
└─────────┘
```

### Rounded Box
```
╭─────────╮
│ content │
╰─────────╯
```

### Box with Subdivisions
```
┌─────┬─────┐
│  A  │  B  │
├─────┼─────┤
│  C  │  D  │
└─────┴─────┘
```

### Tree Structure
```
├─ item 1
│  ├─ sub-item
│  └─ sub-item
└─ item 2
```
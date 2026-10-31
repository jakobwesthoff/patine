# Tables

Plain:

| a | b |
|---|---|
| 1 | 2 |

Aligned columns:

| left | center | right |
|:-----|:------:|------:|
| l1 | c1 | r1 |
| a longer left cell | mid | 12345 |

Styled cells and links:

| item | note |
|------|------|
| `code` | *emphasis* and **strong** and ~~gone~~ |
| [link](https://example.com/a/very/long/path/to/a/page) | ![alt text](https://example.com/image.png) |

Long text that wraps inside its cell:

| topic | description |
|-------|-------------|
| wrapping | This cell holds a long sentence that has to wrap onto several lines when the table is narrow. |
| short | ok |

Many columns of short words (records layout when narrow):

| one | two | three | four | five | six | seven | eight |
|-----|-----|-------|------|------|-----|-------|-------|
| a | b | c | d | e | f | g | h |
| ii | jj | kk | ll | mm | nn | oo | pp |

A word wider than any column:

| id | url |
|----|-----|
| 1 | https://example.com/averyveryverylongpathwithoutanyseparatorthatcannotwrap |

Empty cells and a header only table:

| h1 | | h3 |
|----|--|----|
| | x | |

| only | header |
|------|--------|

Soft break and hard break in a cell:

| a<br>b | c |
|--------|---|
| line one\
line two | d |

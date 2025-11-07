# Mixed-Language Test Corpus

This directory contains files from multiple languages that share common keywords to test cross-language keyword detection.

## Expected Symbol Counts

### Searching for "class" with --symbols (no --lang filter)

Should find classes from ALL languages that support the "class" keyword:

- PHP: 2 classes (PhpUser, PhpProduct)
- TypeScript: 2 classes (TsUser, TsProduct)
- JavaScript: 2 classes (JsUser, JsProduct)
- Python: 2 classes (PyUser, PyProduct)
- Java: 2 classes (JavaUser, JavaProduct)

**Total: 10 classes**

### Searching for "class" with --symbols --lang php

Should find only PHP classes:
- PhpUser
- PhpProduct

**Total: 2 classes**

### Test Cases

1. **Cross-language keyword query**:
   ```bash
   reflex query "class" --symbols
   ```
   Expected: 10 classes from 5 different languages

2. **Language-filtered keyword query**:
   ```bash
   reflex query "class" --symbols --lang php
   ```
   Expected: 2 PHP classes only

3. **Verify keyword mode triggers without --lang**:
   ```bash
   reflex query "class" --symbols
   ```
   Should use keyword mode (scan all files), not trigram search

## Edge Cases Covered

- Cross-language keyword detection
- Same keyword in multiple languages
- Language filtering with keywords
- Keyword mode without language specification

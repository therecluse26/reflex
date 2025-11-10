#!/usr/bin/env bash
#
# Claude Code Benchmark Test Harness
#
# This script helps test rfx vs built-in tools by:
# 1. Running sample queries with both approaches
# 2. Measuring execution time and output size
# 3. Generating comparison logs
#
# Usage:
#   ./run_benchmark.sh [--category CATEGORY] [--test TEST_ID]
#
# Examples:
#   ./run_benchmark.sh                    # Run all tests
#   ./run_benchmark.sh --category 1       # Run Category 1 tests only
#   ./run_benchmark.sh --test 1.1         # Run specific test

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_FILE="$RESULTS_DIR/benchmark_${TIMESTAMP}.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Ensure results directory exists
mkdir -p "$RESULTS_DIR"

# Banner
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Reflex Claude Code Benchmark Suite  ${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if rfx is available
if ! command -v rfx &> /dev/null; then
    echo -e "${RED}ERROR: rfx command not found${NC}"
    echo "Please install reflex or add it to your PATH"
    exit 1
fi

# Check if reflex index exists
if [ ! -d "$REPO_ROOT/.reflex" ]; then
    echo -e "${YELLOW}No reflex index found. Creating index...${NC}"
    cd "$REPO_ROOT"
    rfx index
    echo -e "${GREEN}Index created successfully${NC}"
    echo ""
fi

# Test runner function
run_test() {
    local test_id="$1"
    local test_name="$2"
    local rfx_command="$3"
    local category="$4"
    local complexity="$5"

    echo -e "${BLUE}Running Test $test_id: $test_name${NC}"
    echo "Command: $rfx_command"

    # Run the rfx command and measure time
    local start_time=$(date +%s%3N)
    local output
    local exit_code=0

    cd "$REPO_ROOT"
    output=$(eval "$rfx_command" 2>&1) || exit_code=$?

    local end_time=$(date +%s%3N)
    local duration=$((end_time - start_time))

    # Calculate output size
    local output_size=${#output}
    local line_count=$(echo "$output" | wc -l)

    # Determine status
    local status="PASS"
    if [ $exit_code -ne 0 ]; then
        status="FAIL"
    fi

    # Output results
    echo -e "  Status: ${GREEN}$status${NC}"
    echo -e "  Duration: ${duration}ms"
    echo -e "  Output Size: ${output_size} bytes ($line_count lines)"
    echo ""

    # Escape command for JSON (replace backslashes and quotes)
    local escaped_command=$(echo "$rfx_command" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g')

    # Append to results JSON
    cat >> "$RESULTS_FILE" <<EOF
{
  "test_id": "$test_id",
  "test_name": "$test_name",
  "category": "$category",
  "complexity": "$complexity",
  "command": "$escaped_command",
  "duration_ms": $duration,
  "output_size_bytes": $output_size,
  "output_lines": $line_count,
  "status": "$status",
  "exit_code": $exit_code,
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
},
EOF
}

# Initialize results file
echo "[" > "$RESULTS_FILE"

# Category 1: Simple Text Search
echo -e "${YELLOW}=== Category 1: Simple Text Search ===${NC}"
run_test "1.1" "Find Function Occurrences" \
    "rfx query 'extract_symbols' --json" \
    "Simple Text Search" "Simple"

run_test "1.2" "Find TODO Comments" \
    "rfx query 'TODO' --json" \
    "Simple Text Search" "Simple"

run_test "1.3" "Find Method Calls" \
    "rfx query 'unwrap()' --lang rust --json" \
    "Simple Text Search" "Simple"

run_test "1.4" "Find Public Functions" \
    "rfx query 'pub fn' --lang rust --json" \
    "Simple Text Search" "Simple"

run_test "1.5" "Find Type References" \
    "rfx query 'SymbolKind' --json" \
    "Simple Text Search" "Simple"

# Category 2: Symbol-Aware Search
echo -e "${YELLOW}=== Category 2: Symbol-Aware Search ===${NC}"
run_test "2.1" "Find Function Definition" \
    "rfx query 'extract_symbols' --symbols --json" \
    "Symbol-Aware Search" "Medium"

run_test "2.2" "Find All Struct Definitions" \
    "rfx query 'struct' --kind Struct --json" \
    "Symbol-Aware Search" "Medium"

run_test "2.3" "Find Specific Struct" \
    "rfx query 'SearchResult' --kind Struct --json" \
    "Symbol-Aware Search" "Simple"

run_test "2.4" "Find Functions in Module" \
    "rfx query 'fn' --symbols --glob 'src/parsers/*.rs' --json" \
    "Symbol-Aware Search" "Medium"

run_test "2.5" "Find All Enums" \
    "rfx query 'enum' --kind Enum --json" \
    "Symbol-Aware Search" "Simple"

run_test "2.6" "Find All Traits" \
    "rfx query 'trait' --kind Trait --json" \
    "Symbol-Aware Search" "Simple"

run_test "2.7" "Find Specific Enum" \
    "rfx query 'Language' --kind Enum --json" \
    "Symbol-Aware Search" "Simple"

# Category 3: Glob Pattern Filtering
echo -e "${YELLOW}=== Category 3: Glob Pattern Filtering ===${NC}"
run_test "3.1" "Search in Test Files" \
    "rfx query 'test' --glob '**/*test*.rs' --json" \
    "Glob Pattern Filtering" "Medium"

run_test "3.2" "Search in Specific Directory" \
    "rfx query 'IndexConfig' --glob 'src/**/*.rs' --json" \
    "Glob Pattern Filtering" "Simple"

run_test "3.3" "Search in Parser Files" \
    "rfx query 'parser' --glob 'src/parsers/*.rs' --json" \
    "Glob Pattern Filtering" "Simple"

run_test "3.4" "Search in Examples" \
    "rfx query 'cache' --glob 'examples/*.rs' --json" \
    "Glob Pattern Filtering" "Simple"

run_test "3.5" "Exclude Test Files" \
    "rfx query 'trigram' --exclude '**/*test*.rs' --json" \
    "Glob Pattern Filtering" "Medium"

# Category 4: Language-Specific Filtering
echo -e "${YELLOW}=== Category 4: Language-Specific Filtering ===${NC}"
run_test "4.1" "Python Classes" \
    "rfx query 'class' --lang python --kind Class --glob 'tests/corpus/**/*.py' --json" \
    "Language-Specific Filtering" "Medium"

run_test "4.2" "JavaScript Functions" \
    "rfx query 'function' --lang javascript --json" \
    "Language-Specific Filtering" "Medium"

run_test "4.3" "TypeScript Interfaces" \
    "rfx query 'interface' --kind Interface --lang typescript --json" \
    "Language-Specific Filtering" "Medium"

# Category 5: Paths-Only Mode
echo -e "${YELLOW}=== Category 5: Paths-Only Mode ===${NC}"
run_test "5.1" "Files with TODOs" \
    "rfx query 'TODO' --paths --json" \
    "Paths-Only Mode" "Simple"

run_test "5.2" "Files Mentioning Trigram" \
    "rfx query 'trigram' --paths --json" \
    "Paths-Only Mode" "Simple"

run_test "5.3" "Files with Public Functions" \
    "rfx query 'pub fn' --paths --json" \
    "Paths-Only Mode" "Simple"

# Category 7: Regex Pattern Matching
echo -e "${YELLOW}=== Category 7: Regex Pattern Matching ===${NC}"
run_test "7.1" "Functions Starting with test_" \
    "rfx query 'test_.*' --json" \
    "Regex Pattern Matching" "Medium"

run_test "7.2" "Config and Cache Variables" \
    "rfx query 'config.*cache' --json" \
    "Regex Pattern Matching" "Medium"

run_test "7.3" "Serde Imports" \
    "rfx query 'use serde' --json" \
    "Regex Pattern Matching" "Simple"

# Category 8: Performance & Scale
echo -e "${YELLOW}=== Category 8: Performance & Scale ===${NC}"
run_test "8.1" "Large Result Set - Functions" \
    "rfx query 'fn' --json" \
    "Performance & Scale" "Simple"

run_test "8.2" "Large Result Set - Structs" \
    "rfx query 'struct' --lang rust --json" \
    "Performance & Scale" "Simple"

run_test "8.3" "Common Type Usage" \
    "rfx query 'Result' --json" \
    "Performance & Scale" "Simple"

# Category 9: Edge Cases & Precision
echo -e "${YELLOW}=== Category 9: Edge Cases & Precision ===${NC}"
run_test "9.1" "Exact Enum Location" \
    "rfx query 'IndexStatus' --kind Enum --json" \
    "Edge Cases & Precision" "Simple"

run_test "9.2" "Keyword in Code" \
    "rfx query 'match' --json" \
    "Edge Cases & Precision" "Medium"

run_test "9.4" "Special Characters" \
    "rfx query 'pub(crate)' --json" \
    "Edge Cases & Precision" "Simple"

# Category 10: Context Quality
echo -e "${YELLOW}=== Category 10: Context Quality ===${NC}"
run_test "10.1" "Impl Blocks with Context" \
    "rfx query 'impl' --json" \
    "Context Quality" "Medium"

run_test "10.2" "Function Call Context" \
    "rfx query 'parse_symbols' --json" \
    "Context Quality" "Simple"

# Category 11: Attribute/Annotation Discovery
echo -e "${YELLOW}=== Category 11: Attribute/Annotation Discovery ===${NC}"
run_test "11.2" "Derive Macros" \
    "rfx query '#\[derive' --json" \
    "Attribute/Annotation Discovery" "Simple"

# Close JSON array (remove trailing comma from last entry)
sed -i '$ s/,$//' "$RESULTS_FILE"
echo "]" >> "$RESULTS_FILE"

# Summary
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Benchmark Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "Results saved to: ${BLUE}$RESULTS_FILE${NC}"
echo ""

# Automatically run analysis
echo -e "${BLUE}Running analysis...${NC}"
echo ""

if command -v python3 &> /dev/null; then
    python3 "$SCRIPT_DIR/analyze_results.py" "$RESULTS_FILE"

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Analysis Complete!${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Raw results: $RESULTS_FILE"
    echo ""
    echo "Next steps:"
    echo "  • Review REALISTIC_EXPECTATIONS.md for why your real-world results differ"
    echo "  • Review QUICK_REFERENCE.md for which queries save the most tokens"
    echo "  • Try manual A/B testing with symbol-aware queries for best results"
    echo ""
else
    echo -e "${RED}WARNING: python3 not found. Skipping analysis.${NC}"
    echo "Install Python 3 to see automatic analysis."
    echo ""
    echo "Manual steps:"
    echo "  1. Review results: cat $RESULTS_FILE | jq ."
    echo "  2. Analyze results: python3 $SCRIPT_DIR/analyze_results.py $RESULTS_FILE"
    echo ""
fi

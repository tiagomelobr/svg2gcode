#!/bin/bash

# SVG2GCode Comprehensive Test Runner (Bash version)
# This script tests polygon arc detection functionality across multiple SVG files and settings

set -e

# Configuration
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TESTING_DIR="$WORKSPACE_ROOT/testing"
SVG_SAMPLES_DIR="$TESTING_DIR/svg-samples"
SETTINGS_DIR="$TESTING_DIR/settings"
OUTPUT_DIR="$TESTING_DIR/output"
SCRIPTS_DIR="$TESTING_DIR/scripts"

# Default parameters
TEST_CATEGORY="all"
VERBOSE=false
CLEAN_OUTPUT=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --category)
            TEST_CATEGORY="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --clean)
            CLEAN_OUTPUT=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--category all|functional|performance] [--verbose] [--clean]"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Ensure we're in the right directory
cd "$WORKSPACE_ROOT"

echo "=== SVG2GCode Polygon Arc Detection Test Suite ==="
echo "Workspace: $WORKSPACE_ROOT"
echo "Test Category: $TEST_CATEGORY"

# Clean output directory if requested
if [ "$CLEAN_OUTPUT" = true ]; then
    echo "Cleaning output directory..."
    rm -rf "$OUTPUT_DIR"/*
fi

# Create output directories
for dir in with_arcs without_arcs performance comparison; do
    mkdir -p "$OUTPUT_DIR/$dir"
done

# Build the project first
echo "Building svg2gcode..."
if ! cargo build --release --bin svg2gcode; then
    echo "Build failed. Aborting tests."
    exit 1
fi

# Function to run G-code generation
generate_gcode() {
    local svg_path="$1"
    local settings_path="$2"
    local output_path="$3"
    
    local full_svg_path="$SVG_SAMPLES_DIR/$svg_path"
    local full_settings_path="$SETTINGS_DIR/$settings_path"
    
    if [ ! -f "$full_svg_path" ]; then
        echo "Warning: SVG file not found: $full_svg_path"
        return 1
    fi
    
    if [ ! -f "$full_settings_path" ]; then
        echo "Warning: Settings file not found: $full_settings_path"
        return 1
    fi
    
    if [ "$VERBOSE" = true ]; then
        echo "  Running: svg2gcode '$full_svg_path' --settings '$full_settings_path' --out '$output_path'"
    fi
    
    if "$WORKSPACE_ROOT/target/release/svg2gcode" "$full_svg_path" --settings "$full_settings_path" --out "$output_path" 2>/dev/null; then
        if [ -f "$output_path" ]; then
            return 0
        fi
    fi
    
    echo "Warning: Failed to generate G-code for $svg_path"
    return 1
}

# Function to analyze G-code file
analyze_gcode() {
    local file_path="$1"
    
    if [ ! -f "$file_path" ]; then
        echo "Error: File not found: $file_path"
        return 1
    fi
    
    local total_lines=$(wc -l < "$file_path")
    local g0_count=$(grep -c "^G0" "$file_path" || echo 0)
    local g1_count=$(grep -c "^G1" "$file_path" || echo 0)
    local g2_count=$(grep -c "^G2" "$file_path" || echo 0)
    local g3_count=$(grep -c "^G3" "$file_path" || echo 0)
    
    local movement_commands=$((g0_count + g1_count + g2_count + g3_count))
    local arc_commands=$((g2_count + g3_count))
    local arc_percentage=0
    
    if [ $movement_commands -gt 0 ]; then
        arc_percentage=$(echo "scale=1; $arc_commands * 100 / $movement_commands" | bc -l)
    fi
    
    local file_size_kb=$(du -k "$file_path" | cut -f1)
    
    echo "$total_lines,$g0_count,$g1_count,$g2_count,$g3_count,$arc_commands,$movement_commands,$arc_percentage,$file_size_kb"
}

# Test configurations
declare -a test_configs=(
    "Arc Detection Enabled,polygon_arc_enabled.json,_with_arcs,with_arcs"
    "Arc Detection Disabled,polygon_arc_disabled.json,_without_arcs,without_arcs"
)

# SVG test files with categories
declare -a svg_files=(
    "polygonal/svg-1.svg,polygonal,High"
    "polygonal/svg-2.svg,polygonal,Medium"
    "polygonal/test_circles.svg,polygonal,High"
    "mixed/test_snakes.svg,mixed,Low"
)

# Main test execution
echo ""
echo "=== Running Functional Tests ==="

results_file="$OUTPUT_DIR/test_results.csv"
echo "SvgFile,Configuration,Success,ArcPercentage,ArcCount,FileSizeKB,TotalLines,Category,ExpectedArcs" > "$results_file"

for config in "${test_configs[@]}"; do
    IFS=',' read -r config_name settings_file output_suffix output_dir <<< "$config"
    
    echo ""
    echo "Testing with: $config_name"
    
    for svg in "${svg_files[@]}"; do
        IFS=',' read -r svg_path category expected_arcs <<< "$svg"
        
        svg_name=$(basename "$svg_path" .svg)
        output_file="$OUTPUT_DIR/$output_dir/${svg_name}${output_suffix}.gcode"
        
        echo "  Processing: $svg_path"
        
        if generate_gcode "$svg_path" "$settings_file" "$output_file"; then
            stats=$(analyze_gcode "$output_file")
            IFS=',' read -r total_lines g0_count g1_count g2_count g3_count arc_count movement_count arc_percentage file_size_kb <<< "$stats"
            
            echo "$svg_path,$config_name,true,$arc_percentage,$arc_count,$file_size_kb,$total_lines,$category,$expected_arcs" >> "$results_file"
            
            if [ "$VERBOSE" = true ]; then
                echo "    ✓ Generated $total_lines lines, $arc_percentage% arcs"
            fi
        else
            echo "$svg_path,$config_name,false,0,0,0,0,$category,$expected_arcs" >> "$results_file"
        fi
    done
done

# Performance comparison
if [ "$TEST_CATEGORY" = "all" ] || [ "$TEST_CATEGORY" = "performance" ]; then
    echo ""
    echo "=== Performance Comparison ==="
    
    performance_file="$OUTPUT_DIR/performance_results.csv"
    echo "SvgFile,WithArcsKB,WithoutArcsKB,SizeReductionPercent,ArcPercentage" > "$performance_file"
    
    for svg in "${svg_files[@]}"; do
        IFS=',' read -r svg_path category expected_arcs <<< "$svg"
        svg_name=$(basename "$svg_path" .svg)
        
        with_arcs_file="$OUTPUT_DIR/with_arcs/${svg_name}_with_arcs.gcode"
        without_arcs_file="$OUTPUT_DIR/without_arcs/${svg_name}_without_arcs.gcode"
        
        if [ -f "$with_arcs_file" ] && [ -f "$without_arcs_file" ]; then
            with_stats=$(analyze_gcode "$with_arcs_file")
            without_stats=$(analyze_gcode "$without_arcs_file")
            
            IFS=',' read -r _ _ _ _ _ _ _ with_arc_percentage with_file_size_kb <<< "$with_stats"
            IFS=',' read -r _ _ _ _ _ _ _ _ without_file_size_kb <<< "$without_stats"
            
            size_reduction=0
            if [ "$without_file_size_kb" -gt 0 ]; then
                size_reduction=$(echo "scale=1; ($without_file_size_kb - $with_file_size_kb) * 100 / $without_file_size_kb" | bc -l)
            fi
            
            echo "$svg_path,$with_file_size_kb,$without_file_size_kb,$size_reduction,$with_arc_percentage" >> "$performance_file"
        fi
    done
    
    echo "Performance Results:"
    column -t -s ',' "$performance_file"
fi

# Generate summary report
echo ""
echo "=== Test Summary ==="

successful_tests=$(grep -c ",true," "$results_file" || echo 0)
total_tests=$(tail -n +2 "$results_file" | wc -l)

echo "Tests Passed: $successful_tests / $total_tests"

# Arc detection effectiveness
echo ""
echo "Arc Detection Effectiveness:"
grep "Arc Detection Enabled,true" "$results_file" | while IFS=',' read -r svg_file config success arc_percentage arc_count file_size_kb total_lines category expected_arcs; do
    if [ $(echo "$arc_percentage > 30" | bc -l) -eq 1 ]; then
        color="\033[32m"  # Green
    elif [ $(echo "$arc_percentage > 10" | bc -l) -eq 1 ]; then
        color="\033[33m"  # Yellow
    else
        color="\033[31m"  # Red
    fi
    echo -e "  $svg_file: ${color}${arc_percentage}% arcs detected\033[0m"
done

# Validation checks
echo ""
echo "=== Validation Results ==="

validation_errors=0

# Check that arc-enabled configs actually produce arcs for polygonal SVGs
while IFS=',' read -r svg_file config success arc_percentage arc_count file_size_kb total_lines category expected_arcs; do
    if [ "$category" = "polygonal" ] && [ "$config" = "Arc Detection Enabled" ] && [ "$success" = "true" ]; then
        if [ $(echo "$arc_percentage < 5" | bc -l) -eq 1 ]; then
            echo "  - Low arc detection for polygonal SVG: $svg_file (only $arc_percentage%)"
            validation_errors=$((validation_errors + 1))
        fi
    fi
done < <(tail -n +2 "$results_file")

# Check that arc-disabled configs don't produce many arcs
while IFS=',' read -r svg_file config success arc_percentage arc_count file_size_kb total_lines category expected_arcs; do
    if [ "$config" = "Arc Detection Disabled" ] && [ "$success" = "true" ]; then
        if [ $(echo "$arc_percentage > 5" | bc -l) -eq 1 ]; then
            echo "  - Unexpected arcs when disabled: $svg_file ($arc_percentage%)"
            validation_errors=$((validation_errors + 1))
        fi
    fi
done < <(tail -n +2 "$results_file")

if [ $validation_errors -eq 0 ]; then
    echo "✓ All validation checks passed"
else
    echo "✗ $validation_errors validation errors found"
fi

echo ""
echo "=== Test Complete ==="
echo "Generated files available in: $OUTPUT_DIR"
echo "Test results saved to: $results_file"

# Return appropriate exit code
if [ $validation_errors -eq 0 ] && [ "$successful_tests" -eq "$total_tests" ]; then
    exit 0
else
    exit 1
fi

// WASM Schema Validation Script
// Validates that the WASM interface includes all polygon arc detection fields

const fs = require('fs');
const path = require('path');

// Configuration
const wasmPkgPath = path.join(__dirname, '../../crates/svg2gcode-wasm/pkg');
const wasmModulePath = path.join(wasmPkgPath, 'svg2gcode_wasm_bg.wasm');
const wasmJsPath = path.join(wasmPkgPath, 'svg2gcode_wasm.js');

console.log('=== WASM Schema Validation ===');
console.log('Validating polygon arc detection fields in WASM interface...\n');

// Check if WASM package exists
if (!fs.existsSync(wasmPkgPath)) {
    console.error('❌ WASM package not found. Run: wasm-pack build --target web --out-dir pkg ../crates/svg2gcode-wasm');
    process.exit(1);
}

if (!fs.existsSync(wasmModulePath)) {
    console.error('❌ WASM module not found:', wasmModulePath);
    process.exit(1);
}

if (!fs.existsSync(wasmJsPath)) {
    console.error('❌ WASM JS wrapper not found:', wasmJsPath);
    process.exit(1);
}

try {
    // Import the WASM module
    const wasmBuffer = fs.readFileSync(wasmModulePath);
    const { param_schema_json, initSync } = require(wasmJsPath);
    
    // Initialize WASM
    initSync({ module: wasmBuffer });
    
    // Generate schema
    console.log('🔄 Generating JSON schema...');
    const schemaJson = param_schema_json();
    const schema = JSON.parse(schemaJson);
    
    console.log('✅ Schema generated successfully\n');
    
    // Validation checks
    const errors = [];
    const warnings = [];
    
    // Check for required polygon arc detection fields
    const requiredFields = [
        'detect_polygon_arcs',
        'min_polygon_arc_points', 
        'polygon_arc_tolerance'
    ];
    
    console.log('🔍 Validating polygon arc detection fields...');
    
    requiredFields.forEach(field => {
        if (schema.properties && schema.properties[field]) {
            const fieldSchema = schema.properties[field];
            console.log(`✅ ${field}: Found`);
            
            // Validate field details
            switch (field) {
                case 'detect_polygon_arcs':
                    if (fieldSchema.type !== 'boolean') {
                        errors.push(`${field} should be boolean, got: ${fieldSchema.type}`);
                    }
                    if (fieldSchema.default !== false) {
                        warnings.push(`${field} default should be false, got: ${fieldSchema.default}`);
                    }
                    break;
                    
                case 'min_polygon_arc_points':
                    if (fieldSchema.type !== 'integer') {
                        errors.push(`${field} should be integer, got: ${fieldSchema.type}`);
                    }
                    if (fieldSchema.default !== 5) {
                        warnings.push(`${field} default should be 5, got: ${fieldSchema.default}`);
                    }
                    break;
                    
                case 'polygon_arc_tolerance':
                    if (!Array.isArray(fieldSchema.type) || !fieldSchema.type.includes('number') || !fieldSchema.type.includes('null')) {
                        errors.push(`${field} should be number|null, got: ${JSON.stringify(fieldSchema.type)}`);
                    }
                    if (fieldSchema.default !== null) {
                        warnings.push(`${field} default should be null, got: ${fieldSchema.default}`);
                    }
                    break;
            }
            
            // Check for description
            if (!fieldSchema.description || fieldSchema.description.length < 10) {
                warnings.push(`${field} should have a meaningful description`);
            }
            
        } else {
            errors.push(`Required field missing: ${field}`);
        }
    });
    
    // Check existing fields are still present
    const existingFields = [
        'tolerance',
        'feedrate', 
        'dpi',
        'circular_interpolation',
        'tool_on_sequence',
        'tool_off_sequence'
    ];
    
    console.log('\n🔍 Validating existing fields...');
    existingFields.forEach(field => {
        if (schema.properties && schema.properties[field]) {
            console.log(`✅ ${field}: Present`);
        } else {
            errors.push(`Existing field missing: ${field}`);
        }
    });
    
    // Test schema with sample configuration
    console.log('\n🔄 Testing sample configuration...');
    const sampleConfig = {
        tolerance: 0.002,
        feedrate: 300.0,
        dpi: 96.0,
        circular_interpolation: true,
        detect_polygon_arcs: true,
        min_polygon_arc_points: 5,
        polygon_arc_tolerance: 0.001,
        checksums: false,
        line_numbers: false,
        newline_before_comment: false
    };
    
    try {
        // Simple validation - check if all required fields from schema are satisfied
        const required = schema.required || [];
        const missingRequired = required.filter(field => !(field in sampleConfig));
        
        if (missingRequired.length > 0) {
            errors.push(`Sample config missing required fields: ${missingRequired.join(', ')}`);
        } else {
            console.log('✅ Sample configuration validates successfully');
        }
    } catch (e) {
        errors.push(`Sample configuration validation failed: ${e.message}`);
    }
    
    // Summary
    console.log('\n=== Validation Results ===');
    
    if (errors.length === 0) {
        console.log('✅ All validation checks passed!');
        
        if (warnings.length > 0) {
            console.log('\n⚠️  Warnings:');
            warnings.forEach(warning => console.log(`  - ${warning}`));
        }
        
        console.log('\n📋 Schema Summary:');
        console.log(`  - Total properties: ${Object.keys(schema.properties || {}).length}`);
        console.log(`  - Required fields: ${(schema.required || []).length}`);
        console.log(`  - Polygon arc fields: ${requiredFields.length}/3 present`);
        
        console.log('\n🎉 WASM interface is ready for polygon arc detection!');
        
        // Optionally save schema to file for inspection
        const schemaOutputPath = path.join(__dirname, '../output/current_schema.json');
        fs.mkdirSync(path.dirname(schemaOutputPath), { recursive: true });
        fs.writeFileSync(schemaOutputPath, JSON.stringify(schema, null, 2));
        console.log(`📄 Schema saved to: ${schemaOutputPath}`);
        
        process.exit(0);
        
    } else {
        console.log('❌ Validation failed with errors:');
        errors.forEach(error => console.log(`  - ${error}`));
        
        if (warnings.length > 0) {
            console.log('\n⚠️  Additional warnings:');
            warnings.forEach(warning => console.log(`  - ${warning}`));
        }
        
        process.exit(1);
    }
    
} catch (error) {
    console.error('❌ Validation failed with exception:', error.message);
    console.error('\nTroubleshooting:');
    console.error('1. Ensure WASM package is built: wasm-pack build --target web --out-dir pkg ../crates/svg2gcode-wasm');
    console.error('2. Check that Node.js can load the WASM module');
    console.error('3. Verify all dependencies are installed');
    process.exit(1);
}

# svg2gcode-wasm

WebAssembly (WASM) bindings for the Rust `svg2gcode` library. Convert SVG vector graphics into G-Code directly in browsers or Node.js.

## Install

```bash
npm install svg2gcode-wasm
# or
pnpm add svg2gcode-wasm
# or
yarn add svg2gcode-wasm
```

## Usage (ESM / Bundlers)

```ts
import init, { convert_svg, param_schema_json } from 'svg2gcode-wasm';

// Ensure the WASM module is initialized (most bundlers handle the fetch automatically)
await init();

const svg = `<svg viewBox='0 0 10 10'><rect x='1' y='1' width='8' height='8' stroke='black' fill='none'/></svg>`;

const options = {
  tolerance: 0.002,
  feedrate: 300,
  dpi: 96,
  origin_x: 0,
  origin_y: 0,
  circular_interpolation: false,
  tool_on_sequence: null,
  tool_off_sequence: null,
  begin_sequence: null,
  end_sequence: null,
  between_layers_sequence: null,
  checksums: false,
  line_numbers: false,
  newline_before_comment: false
};

const gcode = convert_svg(svg, options);
console.log(gcode);

// Discover option schema (JSON Schema for dynamic UIs)
console.log(param_schema_json());
```

## API

### `convert_svg(svg: string, options: GCodeConversionOptions) -> string`
Convert SVG markup to a G-Code program string. Throws a string (error message) on failure.

### `param_schema_json() -> string`
Returns a JSON Schema describing the options structure.

## Option Structure

The `options` object flattens three logical groups:

- Conversion: `tolerance`, `feedrate`, `dpi`, `origin_x`, `origin_y`, `extra_attribute_name`
- Machine: `circular_interpolation`, `tool_on_sequence`, `tool_off_sequence`, `begin_sequence`, `end_sequence`, `between_layers_sequence`
- Postprocess: `checksums`, `line_numbers`, `newline_before_comment`

Use `param_schema_json()` if you need to build forms dynamically.

## Building Locally

Requires a Rust toolchain and `wasm-pack`:

```bash
cargo install wasm-pack # if not already installed
npm run build:wasm
```

Outputs artifacts to `pkg/` which are published to npm.

## Versioning

The npm package version tracks the Rust crate version manually. Tag a release (`vX.Y.Z`) to trigger the publish workflow.

## License

MIT

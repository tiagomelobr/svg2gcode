# svg2gcode-wasm

WebAssembly (WASM) bindings for the Rust `svg2gcode` library. Convert SVG vector graphics into G-Code directly in browsers or Node.js.

This fork adds some extra parameters I needed.

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
  newline_before_comment: false,
  // New (dimension / alignment / trim) parameters:
  override_width: "210mm",   // optional length (px, mm, cm, in, etc.)
  override_height: "297mm",  // optional length
  h_align: "center",         // left | center | right (applies if override_* set or trim = true)
  v_align: "bottom",         // top | center | bottom
  trim: true                  // true = scale drawing bbox to fit inside override dims (preserve aspect)
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

Additional layout fields (all optional except `trim` which defaults false):

| Field | Type | Description |
|-------|------|-------------|
| `override_width` | string | Target width with unit (e.g. `210mm`, `8.5in`, `100px`). |
| `override_height` | string | Target height with unit. |
| `h_align` | `"left"|"center"|"right"` | Horizontal alignment within target box / viewport. |
| `v_align` | `"top"|"center"|"bottom"` | Vertical alignment within target box / viewport. |
| `trim` | boolean | Scale drawing’s tight bounding box to fit inside override dims; if only one dimension provided, scales uniformly by that dimension. |

Behavior summary:
* If `trim` is false and overrides are present: overrides define the viewport size; drawing coordinates keep their scale (only alignment translation may occur if size differs).
* If `trim` is true: the drawing bbox is uniformly scaled to fit inside the provided dimensions (paper-fit). Alignment then positions the scaled content.
* Alignment is applied whenever `trim` is true OR any override dimension is provided.

You can introspect the authoritative JSON Schema at runtime via `param_schema_json()` for dynamic form generation.

### TypeScript Helper Interface (example)

```ts
interface GCodeConversionOptions {
  // Conversion
  tolerance: number; feedrate: number; dpi: number;
  origin_x?: number|null; origin_y?: number|null; extra_attribute_name?: string|null;
  // Machine
  circular_interpolation: boolean;
  tool_on_sequence?: string|null; tool_off_sequence?: string|null;
  begin_sequence?: string|null; end_sequence?: string|null; between_layers_sequence?: string|null;
  // Postprocess
  checksums: boolean; line_numbers: boolean; newline_before_comment: boolean;
  // Layout
  override_width?: string; override_height?: string;
  h_align?: 'left'|'center'|'right';
  v_align?: 'top'|'center'|'bottom';
  trim: boolean;
}
```

Use `param_schema_json()` if you need to build forms dynamically or validate user input.

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

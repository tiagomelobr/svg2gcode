# svg2gcode

[![Build, test, and publish coverage for svg2gcode](https://github.com/sameer/svg2gcode/actions/workflows/lib.yml/badge.svg)](https://github.com/sameer/svg2gcode/actions/workflows/lib.yml)

[![Build svg2gcode-cli](https://github.com/sameer/svg2gcode/actions/workflows/cli.yml/badge.svg)](https://github.com/sameer/svg2gcode/actions/workflows/cli.yml)

[![Build svg2gcode-web](https://github.com/sameer/svg2gcode/actions/workflows/web.yml/badge.svg)](https://github.com/sameer/svg2gcode/actions/workflows/web.yml)
[![Deploy svg2gcode-web](https://github.com/sameer/svg2gcode/actions/workflows/web-deploy.yml/badge.svg)](https://github.com/sameer/svg2gcode/actions/workflows/web-deploy.yml)

[![codecov](https://codecov.io/gh/sameer/svg2gcode/branch/master/graph/badge.svg)](https://codecov.io/gh/sameer/svg2gcode)

Convert vector graphics to g-code for pen plotters, laser engravers, and other CNC machines

## Usage

### Web interface

Check it out at https://sameer.github.io/svg2gcode. Just select an SVG and click generate!

![SVG selected on web interface](https://user-images.githubusercontent.com/11097096/129305765-f78da85d-cf4f-4286-a97c-7124a716b5fa.png)

### Command line interface (CLI)

#### Install

```sh
cargo install svg2gcode-cli
```

#### Usage
```
Arguments:
  [FILE]
          A file path to an SVG, else reads from stdin

Options:
      --tolerance <TOLERANCE>
          Curve interpolation tolerance (mm)

      --feedrate <FEEDRATE>
          Machine feed rate (mm/min)

      --dpi <DPI>
          Dots per Inch (DPI) Used for scaling visual units (pixels, points, picas, etc.)

      --on <TOOL_ON_SEQUENCE>
          G-Code for turning on the tool

      --off <TOOL_OFF_SEQUENCE>
          G-Code for turning off the tool

      --begin <BEGIN_SEQUENCE>
          G-Code for initializing the machine at the beginning of the program

      --end <END_SEQUENCE>
          G-Code for stopping/idling the machine at the end of the program

      --between-layers <BETWEEN_LAYERS_SEQUENCE>
          G-Code inserted between sibling SVG group (layer) elements

  -o, --out <OUT>
          Output file path (overwrites old files), else writes to stdout

      --settings <SETTINGS>
          Provide settings from a JSON file. Overrides command-line arguments

      --export <EXPORT>
          Export current settings to a JSON file instead of converting.
          
          Use `-` to export to standard out.

      --origin <ORIGIN>
          Coordinates for the bottom left corner of the machine

      --dimensions <DIMENSIONS>
          Override the width and height of the SVG (i.e. 210mm,297mm)
          
          Useful when the SVG does not specify these (see https://github.com/sameer/svg2gcode/pull/16)
          
          Passing "210mm," or ",297mm" calculates the missing dimension to conform to the viewBox aspect ratio.

      --circular-interpolation <CIRCULAR_INTERPOLATION>
          Whether to use circular arcs when generating g-code
          
          Please check if your machine supports G2/G3 commands before enabling this.
          
          [possible values: true, false]

      --line-numbers <LINE_NUMBERS>
          Include line numbers at the beginning of each line
          
          Useful for debugging/streaming g-code
          
          [possible values: true, false]

      --checksums <CHECKSUMS>
          Include checksums at the end of each line
          
          Useful for streaming g-code
          
          [possible values: true, false]

      --newline-before-comment <NEWLINE_BEFORE_COMMENT>
          Add a newline character before each comment
          
          Workaround for parsers that don't accept comments on the same line
          
          [possible values: true, false]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

#### Example

```sh
svg2gcode-cli examples/Vanderbilt_Commodores_logo.svg --off 'M4' --on 'M5' -o out.gcode
```


To convert curves to G02/G03 Gcode commands, use flag `--circular-interpolation true`.

![Vanderbilt Commodores Logo](examples/Vanderbilt_Commodores_logo.svg)

#### Output, rendered at [https://ncviewer.com](https://ncviewer.com)

```sh
cat out.gcode
```

![Vanderbilt Commodores Logo Gcode](examples/Vanderbilt_Commodores_logo_gcode.png)

### Library

The core functionality of this tool is available as the [svg2gcode crate](https://crates.io/crates/svg2gcode).

## Blog Posts

These go into greater detail on the tool's origins, implementation details, and planned features.

- https://purisa.me/blog/pen-plotter/
- https://purisa.me/blog/svg2gcode-progress/

## FAQ / Interesting details

- Use a 3D printer for plotting: (thanks [@jeevank](https://github.com/jeevank) for sharing this) https://medium.com/@urish/how-to-turn-your-3d-printer-into-a-plotter-in-one-hour-d6fe14559f1a

- Convert a PDF to GCode: follow [this guide using Inkscape to convert a PDF to an SVG](https://en.wikipedia.org/wiki/Wikipedia:Graphics_Lab/Resources/PDF_conversion_to_SVG#Conversion_with_Inkscape), then use it with svg2gcode

- Are shapes, fill patterns supported? No, but you can convert them to paths in Inkscape with `Object to Path`. See [#15](https://github.com/sameer/svg2gcode/issues/15) for more discussion.
- Are stroke patterns supported? No, but you can convert them into paths in Inkscape with `Stroke to Path`.

## Reference Documents

- [W3 SVG2 Specification](https://www.w3.org/TR/SVG/Overview.html)
- [CSS absolute lengths](https://www.w3.org/TR/css-values/#absolute-lengths)
- [CSS font-relative lengths](https://www.w3.org/TR/css-values/#font-relative-lengths)
- [CSS compatible units](https://www.w3.org/TR/css-values/#compat)
- [RepRap G-code](https://reprap.org/wiki/G-code)
- [G-Code and M-Code Reference List for Milling](https://www.cnccookbook.com/g-code-m-code-reference-list-cnc-mills/)

## WASM / JavaScript Usage

An npm package is provided for browser/Node.js usage via WebAssembly.

Install:

```bash
npm install svg2gcode-wasm
```

Example:

```js
import init, { convert_svg } from 'svg2gcode-wasm';
await init();
const gcode = convert_svg('<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" stroke="black" fill="none"/></svg>', {
    tolerance: 0.002,
    feedrate: 300,
    dpi: 96,
    origin_x: 0,
    origin_y: 0,
    // Optional: force a minimum arc radius; if omitted uses tolerance * 0.05
    min_arc_radius: null,
    circular_interpolation: false,
    tool_on_sequence: null,
    tool_off_sequence: null,
    begin_sequence: null,
    end_sequence: null,
    between_layers_sequence: null,
    checksums: false,
    line_numbers: false,
    newline_before_comment: false
});
console.log(gcode);
```

See `crates/svg2gcode-wasm/README.md` for details and advanced usage.

## Release / WASM Publish Workflow

When making a change that requires a new WebAssembly package release, follow this exact process to keep versions consistent and reproducible:

1. Implement and test your changes locally.
2. Bump only the WASM crate version in `crates/svg2gcode-wasm/Cargo.toml` (semantic versioning). Example: `0.1.10 -> 0.1.11`.
3. Run the full build & tests (including examples if relevant):
    - `cargo test`
    - Optionally run a quick conversion sanity check using the CLI or `convert_svg`.
4. Review the working tree: `git status` must show ONLY the intended changes (source files, the WASM `Cargo.toml`, and updated `Cargo.lock`).
5. Stage explicitly (avoid `git add .`):
    - `git add crates/svg2gcode-wasm/Cargo.toml Cargo.lock <changed_source_files>`
6. Commit with a clear message including the new version tag, e.g.: `feat: <summary>; bump wasm to v0.1.11`.
7. Create a lightweight tag matching the WASM version prefixed with a lowercase `v` (format: `vMAJOR.MINOR.PATCH`). Example: `git tag v0.1.11`.
8. Push commit and tag:
    - `git push origin main`
    - `git push origin v0.1.11`
9. (Optional) Publish to npm/crates if part of the release process.

Checklist before tagging:
- [ ] WASM version bumped (and only that crate’s version)
- [ ] Tests pass
- [ ] All required files staged (no unintended changes omitted)
- [ ] Commit message includes the version
- [ ] Tag name exactly matches Cargo version with a leading `v`

Never reuse or move tags; always create a new patch/minor/major version as appropriate.


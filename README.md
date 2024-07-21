# Ekman

Ekman finds the best match positions of overlay images on a background image. It's useful for tasks like identifying UI elements in screenshots or locating specific patterns in larger images.

It was created to find the position of texture assets in [OpenJonsson](https://github.com/kottz/OpenJonsson).

## Usage

Ensure you have Rust installed, then:

```
git clone https://github.com/kottz/ekman.git
cd ekman
cargo run --release -- --background <BACKGROUND_IMAGE> --overlays <OVERLAY_IMAGES> [OPTIONS]
```

Examples:
```
cargo run --release -- --background "background.bmp" --overlays "overlay1.bmp" "overlay2.bmp" --white-transparent
```
```
cargo run --release -- --background "background.bmp" --overlays "overlay*.bmp"
```
```
cargo run --release -- --background "background.bmp" --overlays "overlay*.bmp" --print-format json > output.json
```

Use `--help` for more options.

## Output Formats

- Default
- JSON: Use `--print-format json`

## License

[MIT](LICENSE)

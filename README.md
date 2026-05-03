# collage-maker

A fast CLI tool that arranges multiple images into a single collage.

- **Never crops** input images — only scales
- Minimises gaps with a dynamic-programming row-packing layout
- Outputs **JPG**, **PNG**, or **PDF**
- Defaults to **A4 portrait** at 150 DPI
- Opens the result in your system image viewer automatically

## Install

```sh
# via cargo-binstall (pre-built binary, fastest)
cargo binstall collage-maker

# from source
cargo install collage-maker
```

## Usage

```sh
# All images in a directory → collage.jpg (A4, 150 DPI)
collage photos/

# Explicit files → PDF
collage a.jpg b.jpg c.jpg -o out.pdf

# Custom canvas size
collage photos/ -o out.png --width 2480 --height 3508

# Higher DPI A4
collage photos/ --dpi 300 -o out.jpg

# 8 px gap between images
collage photos/ --gap 8
```

```
USAGE:
    collage [OPTIONS] <INPUTS>...

ARGS:
    <INPUTS>...    Image files or directories (searched recursively)

OPTIONS:
    -o, --output <OUTPUT>    Output file; extension sets format: jpg png pdf [default: collage.jpg]
        --width <WIDTH>      Canvas width in pixels (overrides A4 default)
        --height <HEIGHT>    Canvas height in pixels (overrides A4 default)
        --dpi <DPI>          DPI for A4 size calculation [default: 150]
        --gap <GAP>          Pixel gap between images [default: 4]
    -h, --help               Print help
```

## Layout algorithm

For *n* images the tool tries every row count from 1 to min(*n*, 20).
For each candidate it uses a linear-partition DP (minimises the maximum
per-row aspect-ratio sum) to assign images to rows.  Each row is then
scaled so its images fill the canvas width exactly.  The row count whose
total height is closest to the canvas height wins; if it still overflows,
all rows are scaled down uniformly to fit.

EXIF orientation tags are explicitly ignored — images are placed exactly
as their pixels are stored on disk.

## License

MIT

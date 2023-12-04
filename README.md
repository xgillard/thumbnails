# Thumbnails

Un outil pour accelerer la génération de thumbnails.

## Compilation 
```
cargo build --release
```

## Usage
```
thumbnails 0.1.0
the purpose of this tool is to create image thumbnails in bulk an attempt to maxize the creation throughput

USAGE:
    thumbnails.exe [FLAGS] [OPTIONS] <src> <dst>

FLAGS:
    -a, --asynchronous    Do we want to perform asynchronous io operations ?
        --help            Prints help information
    -V, --version         Prints version information

OPTIONS:
    -e, --extension <extension>    Not all files should be considered when processing the images. Actually, we only want
                                   to process those files having a specific extension and leave out all the others. This
                                   flag allows you to set the only extension to use for that purpose [default: tiff]
    -f, --filter <filter>          The find of filter to use when creating the thumbnails. Can be either of: 'nearest'
                                   (default), 'triangle', 'gaussian', 'catmull-rom', 'lanczos3' The fastest algo is
                                   'nearest' which iterpolates nearest pixels [default: nearest]
    -h, --height <height>          Height of the generated thumbnails [default: 150]
    -w, --width <width>            Width of the generated thumbnails [default: 120]

ARGS:
    <src>    Path to the source folder
    <dst>    Path to the destination folder
```
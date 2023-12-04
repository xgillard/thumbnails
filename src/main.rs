use std::{str::FromStr, path::{PathBuf, Path}, fs, io::Cursor};

use image::ImageOutputFormat;
use rayon::iter::{ParallelIterator, IntoParallelIterator};
use structopt::StructOpt;

/// the purpose of this tool is to create image thumbnails in bulk an attempt to maxize the
/// creation throughput.
#[derive(structopt::StructOpt)]
struct Args {
    /// Path to the source folder
    src: String,
    /// Path to the destination folder
    dst: String,
    /// Width of the generated thumbnails
    #[structopt(short, long, default_value="120")]
    width: u32,
    /// Height of the generated thumbnails
    #[structopt(short, long, default_value="150")]
    height: u32,
    /// Not all files should be considered when processing the images. Actually, we only want to
    /// process those files having a specific extension and leave out all the others. This flag
    /// allows you to set the only extension to use for that purpose.
    #[structopt(short, long, default_value="tif")]
    extension: String,
    /// The find of filter to use when creating the thumbnails. 
    /// Can be either of: 'nearest' (default), 'triangle', 'gaussian', 'catmull-rom', 'lanczos3'
    /// The fastest algo is 'nearest' which iterpolates nearest pixels.
    #[structopt(short, long, default_value="nearest")]
    filter: FilterType, 
    /// Do we want to perform asynchronous io operations ?
    #[structopt(short, long)]
    asynchronous: bool,
}

/// The kind of errors that could potentially happen
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Cannot parse filter type. The only authorized values are 'nearest', 'triangle', 'gaussian', 'catmull-rom', 'lanczos3'")]
    CannotParseFilterType,
    #[error("probleem while processing image {0}")]
    Image(#[from] image::error::ImageError),
    #[error("io error {0}")]
    Io(#[from] std::io::Error),
    #[error("could not join {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

/// Resizes *one* image and save it to the new folder
fn resize_image(input: &[u8], output: &mut Cursor<Vec<u8>>, w: u32, h: u32, f: image::imageops::FilterType) -> Result<(), self::Error>
{  
    let im = image::load_from_memory(input)?;
    let im = image::imageops::resize(&im, w, h, f);
    _ = im.write_to(output, ImageOutputFormat::Jpeg(8))?;
    Ok(())
}

fn sync_version(src: PathBuf, dst: PathBuf, width: u32, height: u32, filter: image::imageops::FilterType) -> Result<(), self::Error>{
    let input = fs::read(src)?;
    let mut output = Cursor::new(vec![]);
    _ = resize_image(&input, &mut output, width, height, filter)?;
    fs::write(dst, output.into_inner())?;
    Ok(())
}

async fn async_version(srcname: PathBuf, dstname: PathBuf, w: u32, h: u32, f: image::imageops::FilterType) -> Result<(), self::Error>{
    let input = tokio::fs::read(srcname).await?;
    
    let output = tokio::task::spawn_blocking(move || async move {
        let mut out = Cursor::new(vec![]);
        _ = resize_image(&input, &mut out, w, h, f).unwrap();
        out
    });

    let output = output.await?.await;

    tokio::fs::write(dstname, output.into_inner()).await?;
    Ok(())
}

fn prepare(src: &str, dst: &str, extension: &str, list: &mut Vec<(PathBuf, PathBuf)>) -> Result<(), Error>{
    if !Path::new(dst).try_exists()? {
        _ = fs::create_dir_all(dst)?;
    }

    let entries = std::fs::read_dir(&src)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let out = PathBuf::from_str(dst).unwrap().join(path.file_name().unwrap().to_str().unwrap());
            _ = prepare(path.to_str().unwrap(), out.to_str().unwrap(), extension, list)?;
        } else {
            let ext = path.extension();
            if let Some(ext) = ext {
                if ext.eq_ignore_ascii_case(extension) {
                    let fstem = path.file_stem().map(|x| x.to_str()).unwrap_or_default().unwrap_or("unk");
                    let dstname = PathBuf::from(&dst).join(format!("{fstem}.jpg"));
                    
                    list.push((path, dstname));
                }
            }
        }
    }

    Ok(())
}


#[tokio::main]
pub async fn main() -> Result<(), self::Error>{
    let Args { src, dst, width, height, extension, filter, asynchronous } = Args::from_args();
    
    let mut list = vec![];
    prepare(&src, &dst, &extension, &mut list)?;

    let f = filter.into();
    if asynchronous {
        let mut tasks = vec![];
        for (s,d) in list {
            let task = tokio::spawn(async_version(s, d, width, height, f));
            tasks.push(task);
        }
        
        for task in tasks {
            _ = task.await?;
        }
    } else {
        list.into_par_iter().for_each(|(s, d)| {
            _ = sync_version(s, d, width, height, f).unwrap();
        });
    }
    
    Ok(())
}

/// Utility cruft to make the cli more user friendly
#[derive(Debug, Clone, Copy)]
enum FilterType {
    Nearest, 
    Triangle, 
    Gaussian, 
    CatmullRom,
    Lanczos3
}
impl FromStr for FilterType {
    type Err = self::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "nearest"     => Ok(Self::Nearest),
            "triangle"    => Ok(Self::Triangle),
            "gaussian"    => Ok(Self::Gaussian),
            "catmull-rom" => Ok(Self::CatmullRom),
            "lanczos3"    => Ok(Self::Lanczos3),
            _             => Err(self::Error::CannotParseFilterType)
        }
    }
}
impl From<FilterType> for image::imageops::FilterType {
    fn from(value: FilterType) -> Self {
        match value {
            FilterType::Nearest    => image::imageops::FilterType::Nearest,
            FilterType::Triangle   => image::imageops::FilterType::Triangle,
            FilterType::Gaussian   => image::imageops::FilterType::Gaussian,
            FilterType::CatmullRom => image::imageops::FilterType::CatmullRom,
            FilterType::Lanczos3   => image::imageops::FilterType::Lanczos3,
        }
    }
}
use std::{str::FromStr, path::{PathBuf, Path}, fs, io::Cursor};

use image::ImageOutputFormat;
use rayon::iter::{ParallelBridge, ParallelIterator};
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

fn par_sync_version(src: &str, dst: &str, width: u32, height: u32, filter: image::imageops::FilterType) -> Result<(), self::Error>{
    let entries = std::fs::read_dir(&src)?;
    entries.par_bridge().for_each(|e| {
        let entry = e.unwrap();
        let srcname = entry.path();

        let fstem = srcname.file_stem().map(|x| x.to_str()).unwrap_or_default().unwrap_or("unk");
        let dstname = PathBuf::from(&dst).join(format!("{fstem}.jpg"));
        
        let input = fs::read(srcname).unwrap();
        let mut output = Cursor::new(vec![]);
        _ = resize_image(&input, &mut output, width, height, filter).unwrap();
        fs::write(dstname, output.into_inner()).unwrap();
    });
    Ok(())
}


async fn full_async_version(src: &str, dst: &str, width: u32, height: u32, filter: image::imageops::FilterType) -> Result<(), self::Error>{
    let mut handles = vec![];
    let mut entries = tokio::fs::read_dir(&src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let srcname = entry.path();

        let fstem = srcname.file_stem().map(|x| x.to_str()).unwrap_or_default().unwrap_or("unk");
        let dstname = PathBuf::from(&dst).join(format!("{fstem}.jpg"));
        
        let handle = tokio::task::spawn(async move {
            _ = process_one(srcname, dstname, width, height, filter).await.unwrap()
        });
        handles.push(handle);
    }
    // join 'em all
    for handle in handles {
        handle.await?;
    }
    Ok(())
}

async fn process_one(srcname: PathBuf, dstname: PathBuf, w: u32, h: u32, f: image::imageops::FilterType) -> Result<(), self::Error>{
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


#[tokio::main]
pub async fn main() -> Result<(), self::Error>{
    let Args { src, dst, width, height, filter, asynchronous } = Args::from_args();
    
    if !Path::new(&dst).exists() {
        std::fs::create_dir(&dst)?;
    }
    if asynchronous {
        full_async_version(&src, &dst, width, height, filter.into()).await?;
    } else {
        par_sync_version(&src, &dst, width, height, filter.into())?;
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
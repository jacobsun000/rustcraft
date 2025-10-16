use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use image::GenericImageView;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args.len() > 4 {
        eprintln!("Usage: atlasify <input.png> <output.json> [tile_size]");
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);
    let tile_size: u32 = if args.len() == 4 {
        args[3].parse().unwrap_or_else(|_| {
            eprintln!("Tile size must be a positive integer");
            std::process::exit(1);
        })
    } else {
        16
    };

    if tile_size == 0 {
        eprintln!("Tile size must be greater than zero");
        std::process::exit(1);
    }

    if !input_path.exists() {
        eprintln!("Input image not found: {}", input_path.display());
        std::process::exit(1);
    }

    let texture_name = input_path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "input path must have a file name",
            )
        })?;

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let target_texture_path = output_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(texture_name);

    if target_texture_path != input_path {
        fs::copy(input_path, &target_texture_path)?;
    }

    let image = image::open(&target_texture_path).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "failed to open image {}: {err}",
                target_texture_path.display()
            ),
        )
    })?;
    let (width, height) = image.dimensions();

    if width % tile_size != 0 || height % tile_size != 0 {
        eprintln!(
            "Image dimensions {}x{} are not divisible by tile size {}",
            width, height, tile_size
        );
        std::process::exit(1);
    }

    let metadata = serde_json::json!({
        "texture": texture_name,
        "tile_size": tile_size,
    });

    let mut file = fs::File::create(output_path)?;
    writeln!(
        file,
        "{}\n",
        serde_json::to_string_pretty(&metadata).unwrap()
    )?;

    println!(
        "Wrote metadata {} (tiles: {} x {})",
        output_path.display(),
        width / tile_size,
        height / tile_size
    );

    Ok(())
}

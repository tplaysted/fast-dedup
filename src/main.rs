// cli imports
use clap::Parser;
use indicatif::{HumanBytes, ProgressBar};

// file system imports
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;

// hashing imports
use fast_dhash::Dhash;
use image;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Copy files to target directory?
    #[arg(short, long, action)]
    target: bool,
}

// Check if a given path points to an image file
fn is_image(path: &Path) -> bool {
    let ext = path.extension();
    if !ext.is_none() {
        match ext.unwrap().to_str() {
            Some("jpg") => true,
            Some("jpeg") => true,
            Some("png") => true,
            _ => false
        }
    } else {
        return false
    }
}

// Index the root directory for all image files
fn get_images_in_dir(dir: &Path) -> io::Result<Vec<DirEntry>> {
    let mut image_paths: Vec<DirEntry> = vec![];
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                match get_images_in_dir(&path) {
                        Err(why) => println!("! {:?}", why.kind()),
                        Ok(paths) => for ent in paths {
                            image_paths.push(ent)
                        },
                    }
            } else {
                if is_image(&entry.path()){
                    image_paths.push(entry)
                }
            }
        }
    }

    return Ok(image_paths);
}

fn generate_hashes(images: &Vec<DirEntry>) -> io::Result<Vec<Dhash>> {
    let mut hashes: Vec<Dhash> = vec![];
    let total: usize = images.len();
    let bar = ProgressBar::new(total.try_into().unwrap());

    for im in images {
        let im_file = image::open(im.path());
        if let Ok(im_file) = im_file {
            hashes.push(Dhash::new(&im_file));
        } 

        bar.inc(1);
    }

    bar.finish();

    return Ok(hashes);
}

fn get_total_size_of_files(images: &Vec<DirEntry>) -> io::Result<u64> {
    let mut total: u64 = 0;

    for im in images {
        total += im.metadata().unwrap().len();
    }

    return Ok(total);
}

fn main() {
    // Get an image and compute its hash
    // let args = Args::parse();

    // Explore the filetree for images
    let root = Path::new(".");
    let images: Vec<DirEntry> = get_images_in_dir(root).unwrap();

    println!("Found {} of image files", HumanBytes(get_total_size_of_files(&images).unwrap()));

    // Generate hashes
    println!("Hashing images...");
    
    let hashes = generate_hashes(&images).unwrap();
}
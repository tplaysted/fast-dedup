// cli imports
use clap::Parser;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle, MultiProgress};

// file system imports
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;

// hashing imports
use fast_dhash::Dhash;
use image;
use std::collections::HashMap;
use std::hash::Hash;

// multithreading imports
use std::sync::{Arc, mpsc};
use std::thread;

// misc imports
use std::time::Duration;
use rand::Rng;

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

// fn get_splits<T>(big_vec: &[T], count: usize) -> Vec<&[T]> {
//     let mut splits = vec![];
//     let r = big_vec.len() % count;
//     let d = big_vec.len() / count;  // len = d * count + r

//     for i in 0..r {
//         splits.push(&big_vec[i * d .. (i + 1) * d + 1]);
//     }

//     for i in r..count {
//         splits.push(&big_vec[i * d .. (i + 1) * d]);
//     } 

//     return splits;
// }

fn generate_hashes(images: &[DirEntry], bar: &ProgressBar) -> io::Result<Vec<Dhash>> {
    let mut hashes: Vec<Dhash> = vec![];

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

fn get_total_size_of_files(images: &[DirEntry]) -> io::Result<u64> {
    let mut total: u64 = 0;

    for im in images {
        total += im.metadata().unwrap().len();
    }

    return Ok(total);
}

fn find_duplicates<'a, K: Eq + Hash + Copy + 'a, V>(keys: &[K], values: &'a [V]) -> (Vec<&'a V>, Vec<&'a V>) {
    let mut originals = vec![];
    let mut duplicates = vec![];
    let mut map: HashMap<K, usize> = HashMap::new();

    for i in 0..std::cmp::min(keys.len(), values.len()) {
        match map.get(&keys[i]) {
            Some(_) => duplicates.push(&values[i]),
            _ => {
                originals.push(&values[i]);
                map.insert(keys[i], i);
            },
        }
    }

    return (originals, duplicates);
}

fn main() {
    // Get an image and compute its hash
    // let args = Args::parse();

    // Explore the filetree for images
    let root = Path::new(".");
    let images = get_images_in_dir(root).unwrap();

    println!("Found {} of image files", HumanBytes(get_total_size_of_files(&images).unwrap()));

    // Progress bar definitions
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("=>-");
    let bar = ProgressBar::new(images.len() as u64);
    bar.set_style(sty);

    // Generate hashes
    println!("Hashing images...");
    
    let hashes = generate_hashes(&images, &bar).unwrap();
    let mut keys = vec![];
    let mut paths = vec![];

    for hash in hashes {
        keys.push(hash.to_u64());
    }

    for im in images {
        let p = im.path();
        let s = p.to_str().unwrap().to_string();
        paths.push(s);
    }

    // find duplicate images
    println!("Finding dupicates...");
    let (orig, dups) = find_duplicates(&keys, &paths);
    println!("Originals: \n");
    for ln in orig {
        println!("{}", ln);
    }
    println!("Duplicates: \n");
    for ln in dups {
        println!("{}", ln);
    }
}


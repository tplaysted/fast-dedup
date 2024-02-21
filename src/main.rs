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
use imagesize;

// multithreading imports
use std::sync::{Arc, mpsc};
use std::thread;

// misc imports
use std::time::Duration;
use rand::Rng;

// Simple command argument struct
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Copy files to target directory?
    #[arg(short, long, action)]
    keep: String,
}

// Implement partial ordering for image paths
trait IsBetterQual {
    fn partial_cmp(&self, other: &Self) -> Option<bool>;
}
impl IsBetterQual for DirEntry {
    fn partial_cmp(&self, other: &Self) -> Option<bool> {
        if !is_image(&self.path()) {return None};
        if !is_image(&other.path()) {return None};

        let self_size: usize;
        let other_size: usize;

        match imagesize::size(self.path()) {
            Ok(dim) => {
                self_size = dim.width * dim.height;
            }
            Err(why) => {println!("Error getting size: {:?}", why); return None;}
        }

        match imagesize::size(other.path()) {
            Ok(dim) => {
                other_size = dim.width * dim.height;
            }
            Err(why) => {println!("Error getting size: {:?}", why); return None;}
        }

        return Some(self_size > other_size);
    }
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

fn find_duplicates<'a, K: Eq + Hash + Copy + 'a, V: IsBetterQual>(keys: &[K], values: &'a [V]) -> (Vec<&'a V>, Vec<&'a V>) {
    let mut originals = vec![];
    let mut duplicates = vec![];
    let mut orig_map: HashMap<K, usize> = HashMap::new();

    for i in 0..std::cmp::min(keys.len(), values.len()) {
        match orig_map.get(&keys[i]) {
            Some(&val_index) => {  // a value already exists at that key
                if values[val_index].partial_cmp(&values[i]).unwrap() { // the new value is better
                    duplicates.push(&values[i]);
                    orig_map.insert(keys[i], val_index);
                } else { // the old value is better
                    duplicates.push(&values[val_index]);
                    orig_map.insert(keys[i], i);
                }
            },
            _ => {
                orig_map.insert(keys[i], i);
            },
        }
    }

    for o in orig_map {  // convert hashmap to vector
        originals.push(&values[o.1]);
    }

    return (originals, duplicates);
}

fn main() {
    // Get an image and compute its hash
    // let args = Args::parse();

    // Explore the filetree for images
    let root = Path::new(".");
    let spin = ProgressBar::new_spinner();
    spin.set_message("Looking for image files...");
    spin.enable_steady_tick(Duration::from_millis(50));

    let images = get_images_in_dir(root).unwrap();
    spin.finish_with_message(format!("Found {} of image files", HumanBytes(get_total_size_of_files(&images).unwrap())));
    
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

    for hash in hashes {
        keys.push(hash.to_u64());
    }

    // find duplicate images
    println!("Finding dupicates...");
    let (orig, dups) = find_duplicates(&keys, &images);
    println!("Originals: \n");
    for ln in orig {
        println!("{:?}", ln);
    }
    println!("Duplicates: \n");
    for ln in dups {
        println!("{:?}", ln);
    }
}


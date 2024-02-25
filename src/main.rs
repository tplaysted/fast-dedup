// cli imports
use clap::{Arg, Command};
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
use std::sync::mpsc;
use std::thread;
use std::thread::available_parallelism;

// misc imports
use std::time::Duration;

// Implement partial ordering for image paths
trait IsBetterQual {
    fn partial_cmp(&self, other: &Self) -> Option<bool>;
}
impl IsBetterQual for Path {
    fn partial_cmp(&self, other: &Self) -> Option<bool> {
        if !is_image(&self) {return None};
        if !is_image(&other) {return None};

        let self_size: usize;
        let other_size: usize;

        match imagesize::size(self) {
            Ok(dim) => {
                self_size = dim.width * dim.height;
            }
            Err(why) => {println!("Error getting size: {:?}", why); return None;}
        }

        match imagesize::size(other) {
            Ok(dim) => {
                other_size = dim.width * dim.height;
            }
            Err(why) => {println!("Error getting size: {:?}", why); return None;}
        }

        return Some(self_size > other_size);
    }
}

impl IsBetterQual for String {
    fn partial_cmp(&self, other: &Self) -> Option<bool> {
        return IsBetterQual::partial_cmp(Path::new(self), Path::new(other));
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
            Some("JPG") => true,
            Some("JPEG") => true,
            Some("PNG") => true,
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

fn get_splits<T: Sized + Clone>(big_vec: Vec<T>, count: usize) -> Vec<Vec<T>> {
    let mut splits = vec![];
    let r = big_vec.len() % count;
    let d = big_vec.len() / count;  // len = d * count + r

    for i in 0..r {
        let mut split = vec![];
        for j in i * d .. (i + 1) * d + 1 {
            split.push(big_vec[j].clone());
        }
        splits.push(split);
    }

    for i in r..count {
        let mut split = vec![];
        for j in i * d .. (i + 1) * d {
            split.push(big_vec[j].clone());
        }
        splits.push(split);
    } 

    return splits;
}

fn generate_hashes(images: Vec<String>, bar: ProgressBar) -> io::Result<Vec<(String, Dhash)>> {
    let mut hashes: Vec<(String, Dhash)> = vec![];

    for im in images {
        let im_file = image::open(Path::new(&im));
        if let Ok(im_file) = im_file {
            hashes.push((im, Dhash::new(&im_file)));
        } 

        bar.inc(1);
    }

    bar.finish_with_message("Done!");

    return Ok(hashes);
}

fn generate_hashes_multithreaded(paths: Vec<String>, sty: ProgressStyle, thread_count: usize) -> io::Result<Vec<(String, Dhash)>> {
    let mut hashes: Vec<(String, Dhash)> = vec![];

    let splits = get_splits(paths, thread_count.try_into().unwrap());

    let (tx, rx) = mpsc::channel();

    let m = MultiProgress::new();
    let mut i = 1;

    for split in splits {
        let new_bar = m.add(ProgressBar::new(split.len().try_into().unwrap()));
        new_bar.set_style(sty.clone());
        new_bar.set_message(format!("Generating hashes, thread #{}", i));
        let tx1 = tx.clone();
        thread::spawn(move || {
            let sub_hashes = generate_hashes(split, new_bar).unwrap();
            for hash in sub_hashes {
                tx1.send(hash).unwrap();
            }
        });
        i += 1;
    }

    drop(tx);

    for received in rx {
        hashes.push(received);
    }

    return Ok(hashes);
}

fn get_total_size_of_files(images: &[DirEntry]) -> io::Result<u64> {
    let mut total: u64 = 0;

    for im in images {
        total += im.metadata().unwrap().len();
    }

    return Ok(total);
}

fn find_duplicates<'a, K: Eq + Hash + Clone + 'a, V: IsBetterQual + Clone>(kvpairs: Vec<(K, V)>) -> (Vec<V>, Vec<V>) {
    let mut keys = vec![];
    let mut values = vec![];
    for pair in kvpairs {
        keys.push(pair.0);
        values.push(pair.1);
    }
    let mut originals = vec![];
    let mut duplicates = vec![];
    let mut orig_map: HashMap<K, usize> = HashMap::new();

    for i in 0..std::cmp::min(keys.len(), values.len()) {
        match orig_map.get(&keys[i]) {
            Some(&val_index) => {  // a value already exists at that key
                if values[val_index].partial_cmp(&values[i]).unwrap() { // the new value is better
                    duplicates.push(values[i].clone());
                    orig_map.insert(keys[i].clone(), val_index);
                } else { // the old value is better
                    duplicates.push(values[val_index].clone());
                    orig_map.insert(keys[i].clone(), i);
                }
            },
            _ => {
                orig_map.insert(keys[i].clone(), i);
            },
        }
    }

    for o in orig_map {  // convert hashmap to vector
        originals.push(values[o.1].clone());
    }

    return (originals, duplicates);
}

fn delete_files(paths: Vec<String>) -> io::Result<()> {
    for item in paths {
        let path = Path::new(&item);
        if path.is_dir() {return Err(std::io::Error::new(std::io::ErrorKind::Other, "Can't delete folder"));}
        if let Err(why) = fs::remove_file(path) {
            return Err(why);
        }
    }

    return Ok(());
}

fn copy_files_to_dir(paths: Vec<String>, dir: &Path) -> io::Result<()> {
    if !dir.is_dir() {return Err(std::io::Error::new(std::io::ErrorKind::Other, "'dir' must be a directory"));}

    for item in paths {
        let path = Path::new(&item);
        if path.is_dir() {return Err(std::io::Error::new(std::io::ErrorKind::Other, "Can't copy folder"));}
        let new_path = dir.join(Path::new(path.file_name().unwrap()));
        let _ = fs::File::create(&new_path).unwrap();
        if let Err(why) = fs::copy(path, new_path) {
            return Err(why);
        }
    }

    return Ok(());
}

fn main() {
    // get cli arguments
    let m = cli().get_matches();

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

    // Generate hashes
    println!("Hashing images...");

    let thread_count: usize;

    if let Some(&t) = m.get_one::<usize>("Threads") {
        let max_threads = available_parallelism().unwrap();
        thread_count = std::cmp::min(t, max_threads.into());
    } else {
        thread_count = 4;
    }
    
    let mut paths = vec![];
    for im in &images {
        paths.push(String::from(im.path().to_str().unwrap()));
    }
    let hashes = generate_hashes_multithreaded(paths, sty, thread_count).unwrap();
    let mut keys = vec![];

    for hash in hashes {
        keys.push((hash.1.to_u64(), hash.0));
    }

    // find duplicate images
    let spin = ProgressBar::new_spinner();
    spin.set_message("Finding dupicates...");
    spin.enable_steady_tick(Duration::from_millis(50));

    let (orig, dups) = find_duplicates(keys);

    spin.finish_with_message(format!("Found {} original images and {} duplicates.", orig.len(), dups.len()));

    // Do copying or deleting
    let spin = ProgressBar::new_spinner();

    if let Some(path) = m.get_one::<String>("Keep") {  // user wants to keep images
        spin.set_message(format!("Copying original images into '{}'", path));
        spin.enable_steady_tick(Duration::from_millis(50));

        if let Err(why) = fs::create_dir(path) {
            spin.finish_with_message(format!("Could not create directory {}: {}", path, why))
        }

        match copy_files_to_dir(orig, Path::new(path)) {
            Ok(_) => spin.finish_with_message(format!("Copied original images into '{}'", path)),
            Err(why) => spin.finish_with_message(format!("Failed to copy images: {}", why))
        }
    } else {
        spin.set_message("Deleting duplicate images...");
        spin.enable_steady_tick(Duration::from_millis(50));

        match delete_files(dups) {
            Ok(_) => spin.finish_with_message("Deleted duplicate images"),
            Err(why) => spin.finish_with_message(format!("Failed to delete duplicate images: {}", why))
        }
    }
}

fn cli() -> Command {
    Command::new("FastDedup")
        .arg(
            Arg::new("Keep")
            .short('k')
            .long("keep")
            .default_missing_value("target")
            .num_args(0..=1)
            .help("Keep files and copy originals into new directory (default '/target')")
        )
        .arg(
            Arg::new("Threads")
            .short('t')
            .long("threads")
            .default_missing_value("4")
            .num_args(0..=1)
            .help("Number of threads to use (default 4)")
            .value_parser(clap::value_parser!(usize))
        )
        .about(
            "A fast utility for removing duplicate image files with perceptual hashing."
        )
}


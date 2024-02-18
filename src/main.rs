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

// multithreading imports
use std::sync::mpsc;
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

fn get_splits<T>(big_vec: &Vec<T>, count: usize) -> Vec<Vec<&T>> {
    let mut splits = vec![];
    for _ in 0..count {
        splits.push(vec![]);
    }

    let mut i = 0;

    for el in big_vec {
        splits[i].push(el);
        if i == count {
            i = 0;
        } else {
            i += 1;
        }
    }

    return splits;
}

fn generate_hashes(images: &Vec<DirEntry>, bar: &ProgressBar) -> io::Result<Vec<Dhash>> {
    let mut hashes: Vec<Dhash> = vec![];
    let total: usize = images.len();

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

fn not_main() {
    // Get an image and compute its hash
    // let args = Args::parse();

    // Explore the filetree for images
    let root = Path::new(".");
    let images: Vec<DirEntry> = get_images_in_dir(root).unwrap();

    println!("Found {} of image files", HumanBytes(get_total_size_of_files(&images).unwrap()));

    // Generate hashes
    println!("Hashing images...");


    // MultiProgress bar definitions
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("=>-");

    // Generate threads
    let thread_count = 4;
    let splits = get_splits(&images, thread_count);

    let mut bars = vec![];
    let mut threads = vec![];

    for _ in 0..thread_count {
        let pb = m.add(ProgressBar::new(images.len().try_into().unwrap()));
        pb.set_style(sty.clone());
        pb.set_message("Hashing...");
        bars.push(pb);
        let t = thread::spawn(move || {

        });
        threads.push(t);
    }


    
    // let hashes = generate_hashes(&images, &bar).unwrap();
}

fn main() {
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");

    let n = 200;
    let pb = m.add(ProgressBar::new(n));
    pb.set_style(sty.clone());
    pb.set_message("todo");
    let pb2 = m.add(ProgressBar::new(n));
    pb2.set_style(sty.clone());
    pb2.set_message("finished");

    let pb3 = m.insert_after(&pb2, ProgressBar::new(1024));
    pb3.set_style(sty);

    m.println("starting!").unwrap();

    let mut threads = vec![];

    let m_clone = m.clone();
    let h3 = thread::spawn(move || {
        for i in 0..1024 {
            thread::sleep(Duration::from_millis(2));
            pb3.set_message(format!("item #{}", i + 1));
            pb3.inc(1);
        }
        m_clone.println("pb3 is done!").unwrap();
        pb3.finish_with_message("done");
    });

    for i in 0..n {
        thread::sleep(Duration::from_millis(15));
        if i == n / 3 {
            thread::sleep(Duration::from_secs(2));
        }
        pb.inc(1);
        let m = m.clone();
        let pb2 = pb2.clone();
        threads.push(thread::spawn(move || {
            let spinner = m.add(ProgressBar::new_spinner().with_message(i.to_string()));
            spinner.enable_steady_tick(Duration::from_millis(100));
            thread::sleep(
                rand::thread_rng().gen_range(Duration::from_secs(1)..Duration::from_secs(5)),
            );
            pb2.inc(1);
        }));
    }
    pb.finish_with_message("all jobs started");

    for thread in threads {
        let _ = thread.join();
    }
    let _ = h3.join();
    pb2.finish_with_message("all jobs done");
    m.clear().unwrap();
}
# fast-dedup

## A CLI utility for fast duplicate image detection and removal

#### How it works:

This tool is based around the [fast-dhash library](https://crates.io/crates/fast-dhash/0.1.0) by Lorenzo Cicuttin. We compute perceptual hashes for all images in a filetree, and use the hashes to detect duplicates. This can be done in linear time by inserting hashes into a hash table - collisions correspond to duplicates. Users can either delete duplicate images, or copy original images into a new directory. 

#### How to install it:

There is no release for this project; you will have to build it from source using the rust build tools. It is also recommended to alias the executable to something convenient.

#### How to use it: 

Simply call `dedup.exe` from the command line.

There are three optional command line arguments. 

- `--keep [<Keep>]`: if present, duplicate images are not deleted. Instead, original images are copied into the provided directory (default is 'target').
- `--threads <Threads>`: by default, fast-dedup will multi-thread the hashing process with a default of 4 threads, but you can override that here. 
- `--help`: print out a help dialogue. 

# fast-dedup

## A CLI utility for fast duplicate image detection and removal

#### How it works:

This tool is based around the fast-dhash library by Lorenzo Cicuttin. We compute perceptual hashes for all images in a filetree, and use the hashes to detect duplicates. This can be done in linear time by inserting hashes into a hash table - collisions correspond to duplicates. Users can either delete duplicate images, or copy original images into a new directory. 

#### How to install it:

There is no release for this project; you will have to build it from source using the rust build tools. It is also recommended to alias the executable to something convenient.

#### How to use it: 

There are three optional command line arguments. 

- '--keep [<Keep>]': If present, duplicate images are not deleted. Instead, original images are copied into the provided directory (default is 'target').
- '--threads <Threads>': By default, fast-dedup will multi-thread the hashing process with a default of 4 threads, but you can override that here. 
- '--help': Print out a help dialogue. 
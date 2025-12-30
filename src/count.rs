use std::thread;
use std::fs::File;

use rayon::prelude::*;
use memmap2::MmapOptions;
#[cfg(unix)]
use memmap2::Advice;
use anyhow::Result;


use crate::bob::Bob;
use crate::bob::Man;

// https://blog.logrocket.com/implementing-data-parallelism-rayon-rust/
// INFO: join must not block - CPU bound task
pub fn rayon_snip() {
    let stores = vec![1,2,3,4,5];
    let _ = stores
        .par_iter() // the rayon magic
        .map(|i| (*i as i32).pow(2))
        .for_each(|i| {
            let tid = thread::current().id();
            // each takes a thread. meaning, expect heavy loads 
            println!("iterated {i} in {:?}", tid);
        });
}

pub fn spawn() {
    let bob = Bob::new(46, String::from("Robert"));
    bob.greet();

    // println!("someone sent us {:?}", bob);
}

pub fn mmap_blak3() -> Result<String> {
    let file = File::open("BLAKE")?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    // WARN: memmap2 does not correctly implement memory mapping with sequential read strategy.
    // require custom solution for later.
    // why? sequential read on mmap don't get `free` to OOM.
    #[cfg(unix)]
    mmap.advise(Advice::Sequential);

    let hash = blake3::hash(&mmap);
    Ok(hash.to_hex().to_string())
}

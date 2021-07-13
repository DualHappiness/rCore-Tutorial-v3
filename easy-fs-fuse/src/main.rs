use std::path::Path;
use std::{
    ffi::OsStr,
    fs::{read_dir, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use clap::Clap;
use easy_fs::{BlockDevice, EasyFileSystem, BLOCK_SIZE};

#[derive(Clap)]
#[clap(name = "EasyFileSystem packer")]
struct Opts {
    #[clap(short, long, about = "Executable source dir(with backslash)")]
    source: String,
    #[clap(short, long, about = "Executable target dir(with backslash)")]
    target: String,
}

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Error when seeking!");
        assert_eq!(
            file.write(buf).unwrap(),
            BLOCK_SIZE,
            "Not a complete block!"
        );
    }
}

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
}

fn easy_fs_pack() -> std::io::Result<()> {
    let opts = Opts::parse();
    let source_path = opts.source;
    let target_path = opts.target;
    println!(
        "source_path = {}\n target_path = {}",
        source_path, target_path
    );
    let total_blocks = 8192u32;
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(total_blocks as u64 * 512).unwrap();
        f
    })));
    // 4MiB, at most 4095 files
    let efs = EasyFileSystem::create(block_file.clone(), total_blocks, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    let apps = read_dir(source_path)
        .unwrap()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .map(|dir| {
            PathBuf::from(dir.file_name())
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        });
    // .map(|str| str.into());
    for app in apps {
        // load app data from host file system
        let mut host_file = File::open(format!("{}{}", target_path, &app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in easy-fs
        let inode = root_inode.create(&app).unwrap();
        // write data to easy-fs
        inode.write_at(0, all_data.as_slice());
    }
    root_inode.ls().iter().for_each(|app| println!("{}", app));
    Ok(())
}

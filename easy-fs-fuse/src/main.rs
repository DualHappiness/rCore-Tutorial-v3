use std::{
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

    fn get_dev_id(&self) -> usize {
        0
    }
}

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
}

const TOTAL_BLOCKS: u32 = 8192;

fn easy_fs_pack() -> std::io::Result<()> {
    let opts = Opts::parse();
    let source_path = opts.source;
    let target_path = opts.target;
    println!(
        "source_path = {}\n target_path = {}",
        source_path, target_path
    );
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(TOTAL_BLOCKS as u64 * 512).unwrap();
        f
    })));
    // 4MiB, at most 4095 files
    let efs = EasyFileSystem::create(block_file.clone(), TOTAL_BLOCKS, 1);
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
        let filename = format!("{}{}.elf", target_path, &app);
        // load app data from host file system
        if let Ok(mut host_file) = File::open(filename) {
            let mut all_data: Vec<u8> = Vec::new();
            host_file.read_to_end(&mut all_data).unwrap();
            // create a file in easy-fs
            let inode = root_inode.create(&app).unwrap();
            // write data to easy-fs
            inode.write_at(0, all_data.as_slice());
        }
    }
    root_inode.ls().iter().for_each(|app| println!("{}", app));
    Ok(())
}

#[test]
fn efs_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(TOTAL_BLOCKS as u64 * 512).unwrap();
        f
    })));
    EasyFileSystem::create(block_file.clone(), TOTAL_BLOCKS, 1);
    let efs = EasyFileSystem::open(block_file.clone());
    let root_inode = EasyFileSystem::root_inode(&efs);

    root_inode.create("filea");
    root_inode.create("fileb");
    root_inode.ls().iter().for_each(|name| println!("{}", name));
    let filea = root_inode.find("filea").unwrap();
    let greet_str = "Hello, World!";
    filea.write_at(0, greet_str.as_bytes());

    let mut buffer = [0u8; 233];
    let len = filea.read_at(0, &mut buffer);
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap());

    let mut random_str_test = |len: usize| {
        filea.clear();
        assert_eq!(filea.read_at(0, &mut buffer), 0);
        let mut str = String::new();
        use rand;
        for _ in 0..len {
            str.push(char::from(b'0' + rand::random::<u8>() % 10));
        }
        let _write_len = filea.write_at(0, str.as_bytes());
        let mut read_buffer = [0u8; 127];
        let mut offset = 0usize;
        let mut read_str = String::new();
        loop {
            let len = filea.read_at(offset, &mut read_buffer);
            if len == 0 {
                break;
            }
            offset += len;
            read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
        }
        assert_eq!(str, read_str);
    };

    random_str_test(4 * BLOCK_SIZE);
    random_str_test(8 * BLOCK_SIZE + BLOCK_SIZE / 2);
    random_str_test(100 * BLOCK_SIZE);
    random_str_test(70 * BLOCK_SIZE);
    random_str_test((12 + 128) * BLOCK_SIZE);
    random_str_test(400 * BLOCK_SIZE);
    random_str_test(1000 * BLOCK_SIZE);
    random_str_test(2000 * BLOCK_SIZE);
    Ok(())
}

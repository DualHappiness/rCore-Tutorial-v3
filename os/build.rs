use std::fs::{read_dir, File};
use std::io::{Result, Write};

static TARGET_PATH: &str = "../tests/user/build/bin/";

fn main() {
    println!("cargo:return-if-changed=../user/src/");
    println!("cargo:return-if-changed={}", TARGET_PATH);
    insert_app_data().unwrap();
}

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_app.S")?;
    let mut apps: Vec<_> = read_dir(TARGET_PATH)?
        .into_iter()
        .filter_map(|dir| dir.ok())
        .map(|dir| dir.path().file_stem().unwrap().to_str().unwrap().to_owned())
        .collect();
    apps.sort();
    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(
            f,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "{2}{1}.bin"
app_{0}_end:"#,
            idx, app, TARGET_PATH
        )?;
    }
    Ok(())
}

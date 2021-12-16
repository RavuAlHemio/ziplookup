use std::env;
use std::ffi::OsString;
use std::fs::{File, read_dir};
use std::io::{Cursor, Read};
use std::path::PathBuf;

use zip::ZipArchive;


const MAX_DEPTH: usize = 8;


fn name_is_relevant_archive(name: &str) -> bool {
    let lower_name = name.to_lowercase();
    lower_name.ends_with(".zip")
        || lower_name.ends_with(".jar")
        || lower_name.ends_with(".ear")
        || lower_name.ends_with(".war")
}


fn scan_archive(archive_path: &str, archive_data: &Vec<u8>, search_name_lower: &str, remain_depth: usize, trace_each: usize, trace_counter: &mut usize) {
    let cursor = Cursor::new(archive_data);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("failed to open {:?} as a ZIP archive: {}", archive_path, e);
            return;
        },
    };

    let archive_file_names: Vec<String> = archive.file_names()
        .map(|n| n.to_owned())
        .collect();

    for name in &archive_file_names {
        let entry_path = format!("{}[{}]", archive_path, name);
        if should_trace_this(trace_each, trace_counter) {
            eprintln!("A> {:?}", entry_path);
        }
        let mut entry = match archive.by_name(name) {
            Ok(sae) => sae,
            Err(e) => {
                eprintln!("failed to obtain {:?} from {:?}: {}", name, archive_path, e);
                continue;
            },
        };
        if entry.is_dir() {
            continue;
        }

        if name_is_relevant_archive(name) {
            if remain_depth == 0 {
                eprintln!("{:?} in {:?} is apparently an archive but we have exceeded the maximum depth", name, archive_path);
                continue;
            }

            let mut sub_archive_bytes = Vec::new();
            if let Err(e) = entry.read_to_end(&mut sub_archive_bytes) {
                eprintln!("failed to read {:?} from {:?}: {}", name, archive_path, e);
                continue;
            }
            scan_archive(&entry_path, &sub_archive_bytes, search_name_lower, remain_depth - 1, trace_each, trace_counter);
        } else {
            let name = entry.name();
            let naked_name = match name.find(&['/', '\\'][..]) {
                Some(last_slash_index) => &name[last_slash_index+1..],
                None => name,
            };
            let naked_name_lower = naked_name.to_lowercase();
            if naked_name_lower == search_name_lower {
                println!("{}", entry_path);
            }
        }
    }
}


fn print_usage() {
    eprintln!("Usage: ziplookup [--trace|--trace-some] STARTDIR SEARCHFILENAME");
}


#[inline]
fn should_trace_this(trace_each: usize, trace_counter: &mut usize) -> bool {
    *trace_counter += 1;
    if *trace_counter == trace_each {
        *trace_counter = 0;
        true
    } else {
        false
    }
}


fn run() -> i32 {
    let args_os: Vec<OsString> = env::args_os().collect();
    let mut trace_each = 0;
    let mut pos_start = 1;
    if args_os.len() < 3 {
        print_usage();
        return 1;
    }
    if args_os.get(1).map(|a| a == "--trace").unwrap_or(false) {
        trace_each = 1;
        pos_start = 2;
    } else if args_os.get(1).map(|a| a == "--trace-some").unwrap_or(false) {
        trace_each = 16384;
        pos_start = 2;
    }

    if args_os.len() != pos_start + 2 {
        print_usage();
        return 1;
    }

    let start_dir = args_os.get(pos_start).expect("no start directory given");
    let search_name_lower = args_os.get(pos_start + 1).expect("no search name given")
        .to_str().expect("failed to decode search name as UTF-8")
        .to_lowercase();

    let mut dir_stack = Vec::new();
    dir_stack.push(PathBuf::from(start_dir));

    let mut trace_counter = 0;
    while let Some(dir_path) = dir_stack.pop() {
        if should_trace_this(trace_each, &mut trace_counter) {
            eprintln!("F> {:?}", dir_path);
        }

        let entries = match read_dir(&dir_path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("failed to read entries of {:?}: {}", dir_path, e);
                continue;
            },
        };

        for entry_res in entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("failed to read an entry of {:?}: {}", dir_path, e);
                    continue;
                },
            };

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("failed to read metadata of {:?}: {}", entry.path(), e);
                    continue;
                },
            };
            if metadata.is_dir() {
                // descend
                dir_stack.push(entry.path());
                continue;
            }

            let entry_path = entry.path();
            let entry_path_str = match entry_path.as_os_str().to_str() {
                Some(aps) => aps,
                None => {
                    eprintln!("failed to convert path {:?} to UTF-8 string", entry.path());
                    continue;
                },
            };
            let entry_file_name = entry.file_name();
            let entry_file_name_str = match entry_file_name.to_str() {
                Some(aps) => aps,
                None => {
                    eprintln!("failed to convert file name {:?} of {:?} to UTF-8 string", entry.file_name(), entry_path_str);
                    continue;
                },
            };
            if name_is_relevant_archive(entry_file_name_str) {
                // scan the file as ZIP
                let zip_bytes = {
                    let mut zip_file = match File::open(&entry.path()) {
                        Ok(zf) => zf,
                        Err(e) => {
                            eprintln!("failed to open {:?}: {}", entry.path(), e);
                            continue;
                        },
                    };
                    let mut zb = Vec::new();
                    if let Err(e) = zip_file.read_to_end(&mut zb) {
                        eprintln!("failed to read {:?}: {}", entry.path(), e);
                        continue;
                    }
                    zb
                };
                scan_archive(entry_path_str, &zip_bytes, &search_name_lower, MAX_DEPTH, trace_each, &mut trace_counter);
            } else {
                let file_name_lower = entry_file_name_str.to_lowercase();
                if file_name_lower == search_name_lower {
                    println!("{}", entry_path_str);
                }
            }
        }
    }

    0
}

fn main() {
    std::process::exit(run());
}

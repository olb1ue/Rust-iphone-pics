use std::fs::File;
use std::fs;
use std::env;
use std::path::Path;
use std::io::{Error, ErrorKind};
use std::time::SystemTime;

use chrono::{Datelike, NaiveDateTime, Month, FixedOffset, DateTime, TimeZone};

use exif::{Tag, In};
use ffprobe::ffprobe;

const SELF_NAME: &str = "rust-exif";

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        2 => {
            println!("{}", args[1]);
            let path: &Path = Path::new(&args[1]);
            match path.is_dir() {
                true => {
                    match ls_dir(path) {
                        Ok(_) => println!("Done"),
                        Err(e) =>  println!("{}",  e),
                    }
                },
                false => println!("path not found: {}", args[1]),
            }
            // ls_dir(Path::new("./")).expect("problem with  ls_dir");
        },
        _ => println!("Argument should be path."),
    }

}

fn create_dir(path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}

fn ls_dir(path: &Path) -> std::io::Result<()> {
    for f in path.read_dir()? {
        let f = f?;
        if !f.path().is_dir() {
            // println!("{}", f.path().file_name().and_then(|s| s.to_str()).unwrap());
            // println!("{:?}", f.path().extension());

            let mut dates: Option<DateTime<FixedOffset>> = None;

            let fpath = f.path();
            let ext = match fpath.extension() {
                Some(ex) => ex.to_str(),
                None => continue,
            };
            match ext {
                Some("MOV") => {
                    match ffprobe(f.path()) {
                        Ok(info) => {
                            match info.format.tags {
                                Some(ft) => {
                                    match ft.apple_creation_date {
                                        Some(acd) => dates = Some(DateTime::parse_from_str(acd.as_str(), "%FT%T%z").expect("couldn't create datetime from apple creation date")),
                                        None => println!("No apple creation date found in tags"),
                                    }
                                }
                                None => println!("no tags from ffprobe"),
                            }
                        },
                        Err(e) => eprintln!("ffprobe Error: {:?}", e),
                    }

                },
                Some("") => {
                    println!("Skipping {}", f.path().file_name().unwrap().to_str().unwrap());
                }
                _ => {
                    let file = File::open(f.path())?;
                    let mut bufreader = std::io::BufReader::new(&file);
                    let exifreader = exif::Reader::new();
                    let exif = match exifreader.read_from_container(&mut bufreader) {
                        Ok(ex) => Some(ex),
                        Err(_) => {
                            None
                        },
                    };
                    match exif {                        
                        Some(ex) => match ex.get_field(Tag::DateTimeOriginal, In::PRIMARY) {                            
                            Some(dt) => {
                                if let exif::Value::Ascii(ref vec) = dt.value {
                                    let dt_s = format!("{}", dt.value.display_as(dt.tag));
                                    match NaiveDateTime::parse_from_str(dt_s.as_str(), "%F %T") {
                                        Ok(naive) => dates = Some(add_tz(naive)),
                                        Err(_) => {},
                                    }
                                }
                            },
                            None => {
                                match ex.get_field(Tag::DateTime, In::PRIMARY) {
                                    Some(dt) =>  {
                                        if let exif::Value::Ascii(_) = dt.value {
                                            let dt_s = format!("{}", dt.value.display_as(dt.tag));
                                            match NaiveDateTime::parse_from_str(dt_s.as_str(), "%F %T") {
                                                Ok(naive) => dates = Some(add_tz(naive)),
                                                Err(_) => {},
                                            }
                                        }
                                    },
                                    None  => {},
                                }
                            },
                        },
                        None => {},
                    };


                },
            }
            match dates {
                Some(_) => {},
                None => {
                    let created_date = metadata_created(&fpath.as_os_str().to_str().unwrap())?;
                    let secs = match created_date.duration_since(SystemTime::UNIX_EPOCH) {
                        Ok(s) => s.as_secs(),
                        Err(_)  => panic!("couldn't parse system time"),
                    };
                    let naive =  NaiveDateTime::parse_from_str(&secs.to_string(), "%s").expect("problem parsing timestamp");
                    dates = Some(add_tz(naive));

                }
            }
            let d = dates.unwrap();
            let month_number = d.month();
            let new_path = format!("{}/{}_{}", d.year(), month_number, Month::try_from(u8::try_from(d.month()).unwrap()).unwrap().name());
            let dst_dir = Path::new(&new_path);
            let full_dst = path.join(dst_dir);
            create_dir(&full_dst).expect("Error creating dir: {}");
            let dst_path = full_dst.join(f.file_name());
            fs::rename(&f.path(), &dst_path).expect("Error moving file");
    
        }
    }
    Ok(())
}

// fn get_offset_from_exif(ex: &exif::Exif) -> i16 {
//     match ex.get_field(Tag::OffsetTime, In::PRIMARY) {
//         Some(offset) => {
//             if let exif::Value::Ascii(_) = offset.value {
//                 println!("{}", offset.value.display_as(Tag::OffsetTime));
//                 let n: i16 = offset.value.display_as(Tag::OffsetTime).parse().unwrap();
//                 println!("OK: {}", n);
//                 return n;
//             } else {
//                 println!("got offset but not sshort");
//                 return -0700;
//             }
//         },
//         _ => return -0700,
//     }
// }
fn add_tz(naive: NaiveDateTime) -> DateTime<FixedOffset> {
    let tz_offset: FixedOffset = FixedOffset::west_opt(7 * 3600).unwrap();
    return tz_offset.from_local_datetime(&naive).unwrap();
}
fn metadata_created(path: &str) -> std::io::Result<SystemTime> {
    let metadata = fs::metadata(path)?;
    if let Ok(time) = metadata.modified() {
        Ok(time)
    } else {
        println!("Error getting metadata for file: {}", path);
        Err(Error::new(ErrorKind::Other, "Couldn't get metadata"))
    }
}
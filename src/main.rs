use std::env;
use std::process::exit;
use std::path::Path;
use quick_xml::{Reader, Writer};
use quick_xml::events::{Event, BytesText, BytesStart, BytesEnd};
use chrono::DateTime;
use std::io::{Cursor, Write};
use std::fs::File;

extern crate quick_xml;
extern crate chrono;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        println!("Merci de fournir le nom du fichier, l'altitude de départ et l'altitude d'arrivée");
        exit(-1)
    }

    let filename = &args[1];
    if !Path::new(filename).exists() {
        println!("Le fichier {} n'existe pas", filename);
        exit(-2)
    }
    let elevation_depart = &args[2].parse::<i64>().unwrap_or_else(|_error| {
        println!("L'altitude de départ n'est pas un entier");
        exit(-3)});
    let elevation_arrivee = &args[3].parse::<i64>().unwrap_or_else(|_error| {
        println!("L'altitude d'arrivée n'est pas un entier");
        exit(-4)});;

    let mut reader = Reader::from_file(filename).unwrap();
    reader.trim_text(true);
    // let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut num_point = 0;
    let mut premiere_altitude = 0.0;
    let mut premiere_heure = "".to_string();
    let mut derniere_altitude = 0.0;
    let mut derniere_heure = "".to_string();
    let mut dans_ele = false;
    let mut dans_time= false;
    let mut dans_trk = false;
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == b"trk" => {
                dans_trk = true;
            },
            Ok(Event::End(ref e)) if e.name() == b"trk" => {
                dans_trk = false;
            }
            Ok(Event::Start(ref e)) if e.name() == b"ele" => {
                dans_ele = true;
                num_point += 1;
            },
            Ok(Event::End(ref e)) if e.name() == b"ele" => {
                dans_ele = false;
            },
            Ok(Event::Start(ref e)) if e.name() == b"time" && dans_trk => {
                dans_time = true;
            },
            Ok(Event::End(ref e)) if e.name() == b"time" && dans_trk => {
                dans_time = false;
            },
            Ok(Event::Text(e)) => {
                if dans_ele {
                    if num_point == 1 {
                        premiere_altitude = e.unescape_and_decode(&reader).unwrap().parse().unwrap()
                    } else {
                        derniere_altitude = e.unescape_and_decode(&reader).unwrap().parse().unwrap()
                    }
                } else if dans_time {
                    if num_point == 1 {
                        premiere_heure = e.unescape_and_decode(&reader).unwrap();
                    } else {
                        derniere_heure = e.unescape_and_decode(&reader).unwrap();
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
    }

    let duree_trajet = DateTime::parse_from_rfc3339(&derniere_heure).unwrap().timestamp() -
        DateTime::parse_from_rfc3339(&premiere_heure).unwrap().timestamp();
    let correc_initiale = *elevation_depart as f64 - premiere_altitude;
    let diff_correction = *elevation_arrivee as f64 - derniere_altitude - correc_initiale;

    reader = Reader::from_file(filename).unwrap();
    reader.trim_text(true);
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), 32u8, 2);
    buf = Vec::new();
    dans_ele = false;
    dans_time = false;
    dans_trk = false;
    let mut elevation = 0.0;

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == b"trk" => {
                dans_trk = true;
                let elem = BytesStart::owned(b"trk".to_vec(), "trk".len());
                assert!(writer.write_event(Event::Start(elem)).is_ok());
            },
            Ok(Event::End(ref e)) if e.name() == b"trk" => {
                dans_trk = false;
                let elem = BytesEnd::owned(b"trk".to_vec());
                assert!(writer.write_event(Event::End(elem)).is_ok());
            },
            Ok(Event::Start(ref e)) if e.name() == b"ele" => {
                dans_ele = true;
            },
            Ok(Event::End(ref e)) if e.name() == b"ele" => {
                dans_ele = false;
            },
            Ok(Event::Start(ref e)) if e.name() == b"time" && dans_trk => {
                dans_time = true;
            },
            Ok(Event::End(ref e)) if e.name() == b"time" && dans_trk => {
                dans_time = false;
                let elem = BytesEnd::owned(b"time".to_vec());
                assert!(writer.write_event(Event::End(elem)).is_ok());
            },
            Ok(Event::Text(e)) => {
                if dans_ele {
                    elevation = e.unescape_and_decode(&reader).unwrap().parse().unwrap();
                } else if dans_time {
                    let heure = e.unescape_and_decode(&reader).unwrap();
                    let delai = DateTime::parse_from_rfc3339(&heure).unwrap().timestamp() -
                        DateTime::parse_from_rfc3339(&premiere_heure).unwrap().timestamp();
                    let elevation_corrigee = (elevation as f64 + correc_initiale + diff_correction * delai as f64 / duree_trajet as f64).to_string();
                    let elem = BytesStart::owned(b"ele".to_vec(), "ele".len());
                    assert!(writer.write_event(Event::Start(elem)).is_ok());
                    let elem = BytesText::from_plain_str(&elevation_corrigee);
                    assert!(writer.write_event(Event::Text(elem)).is_ok());
                    let elem = BytesEnd::owned(b"ele".to_vec());
                    assert!(writer.write_event(Event::End(elem)).is_ok());
                    let elem = BytesStart::owned(b"time".to_vec(), "time".len());
                    assert!(writer.write_event(Event::Start(elem)).is_ok());
                    assert!(writer.write_event(Event::Text(e)).is_ok());
                } else {
                    assert!(writer.write_event(Event::Text(e)).is_ok());
                }
            },
            Ok(Event::Eof) => break,
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            // or using the buffer
            // Ok(e) => assert!(writer.write(&buf).is_ok()),
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
        }
    }
    buf.clear();
    let result = String::from_utf8(writer.into_inner().into_inner()).unwrap();
    let file_result = filename.replace(".", " C.");
    let mut file = File::create(file_result).unwrap();
    file.write_all(result.as_bytes()).unwrap();
}
